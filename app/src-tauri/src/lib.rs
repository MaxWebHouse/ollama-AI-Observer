// In src-tauri/src/lib.rs

#![cfg_attr(not(debug_assertions), windows_subsystem = "windows")]

// ---- Final, Corrected Imports ----
use axum::{
    body::Body,
    extract::{Query, State as AxumState},
    http::{HeaderMap, Method, StatusCode, Uri},
    response::Response,
    response::sse::{Event, Sse},
    routing::{any, get},
    Router,
};
use futures::stream::Stream;
use http_body_util::BodyExt;
use reqwest::Client;
use serde::Deserialize;
use std::convert::Infallible;
use std::sync::Mutex;
use tauri::{
    menu::{Menu, MenuItem},
    tray::TrayIconBuilder,
    AppHandle, Manager, State,
};
use tauri_plugin_shell::ShellExt;
use tower_http::{cors::{Any, CorsLayer}, services::ServeDir};
use tokio::{
    self, 
    io::{AsyncBufReadExt, BufReader},
    process::Command as TokioCommand,
};
use futures::future::join_all;
use futures::stream::select as stream_select;

struct AppSettings {
  ollama_url: Mutex<Option<String>>,
}

#[tauri::command]
async fn set_ollama_url(
    new_url: Option<String>, // Can be a string or null from frontend
    settings: State<'_, AppSettings>,
) -> Result<(), String> {
    log::info!("Setting Ollama URL to: {:?}", new_url);
    // Lock the mutex to get exclusive access and update the value.
    *settings.ollama_url.lock().unwrap() = new_url;
    Ok(()) // Return Ok to signal success to the frontend
}

#[tauri::command]
async fn get_ollama_url(
    settings: State<'_, AppSettings>,
) -> Result<Option<String>, String> {
    log::info!("Getting Ollama URL");
    // Lock the mutex, clone the value inside, and return it.
    // We clone so we don't hold the lock longer than necessary.
    let url = settings.ollama_url.lock().unwrap().clone();
    Ok(url)
}

#[tauri::command]
async fn check_ollama_servers(urls: Vec<String>) -> Result<Vec<String>, String> { // <-- No State parameter
    log::info!("Rust backend received request to check servers (using dedicated client): {:?}", urls);

    // Create a new, temporary client just for this operation.
    let client = Client::new();

    // The rest of the logic is identical.
    let checks = urls.into_iter().map(|url| {
        let client = client.clone();
        let check_url = format!("{}/v1/models", url);

        tokio::spawn(async move {
            match client.get(&check_url).timeout(std::time::Duration::from_millis(2500)).send().await {
                Ok(response) if response.status().is_success() => {
                    log::info!("Success checking server at {}", url);
                    Some(url)
                }
                Ok(response) => {
                    log::warn!("Failed check for {}: Status {}", url, response.status());
                    None
                }
                Err(e) => {
                    log::warn!("Failed check for {}: Error: {}", url, e);
                    None
                }
            }
        })
    });

    let results = join_all(checks).await;

    let successful_urls: Vec<String> = results
        .into_iter()
        .filter_map(|res| res.ok().flatten())
        .collect();

    log::info!("Found running servers at: {:?}", successful_urls);

    Ok(successful_urls)
}

// Shared state for our application
#[derive(Clone)]
struct AppState {
    app_handle: AppHandle,
    http_client: Client,
}

#[derive(Debug, Deserialize)]
struct ExecParams {
    cmd: String,
}

async fn exec_handler(
    AxumState(state): AxumState<AppState>,
    Query(params): Query<ExecParams>,
) -> Sse<impl Stream<Item = Result<Event, Infallible>>> {
    log::info!("Received command to execute: '{}'", params.cmd);

    let stream = async_stream::stream! {
        const UNAUTHORIZED_MESSAGE: &str = "[unauthorized]";

        if params.cmd.chars().any(|c| "&;|<>()`$".contains(c)) {
            log::warn!("Unauthorized command blocked: contains forbidden characters ('{}').", params.cmd);
            yield Ok(Event::default().event("error").data(UNAUTHORIZED_MESSAGE));
            return;
        }

        let parts: Vec<&str> = params.cmd.split_whitespace().collect();
        if parts.is_empty() {
            yield Ok(Event::default().event("error").data("Empty command received."));
            return;
        }

        if parts[0] != "ollama" {
            log::warn!("Unauthorized command blocked: only 'ollama' is permitted ('{}').", params.cmd);
            yield Ok(Event::default().event("error").data(UNAUTHORIZED_MESSAGE));
            return;
        }

        let args: &[&str];
        if parts.len() == 1 {
            args = &[];
        } else {
            let subcommand = parts[1];
            let allowed_subcommands = [
                "serve", "create", "show", "run", "stop", "pull",
                "push", "list", "ps", "cp", "rm", "help", "--version", "--help"
            ];
            if !allowed_subcommands.contains(&subcommand) {
                log::warn!("Unauthorized command blocked: subcommand '{}' is not permitted.", subcommand);
                yield Ok(Event::default().event("error").data(UNAUTHORIZED_MESSAGE));
                return;
            }
            args = &parts[1..];
        }

        #[cfg(target_os = "windows")]
        let program = "ollama";

        #[cfg(not(target_os = "windows"))]
        let program = "/usr/local/bin/ollama";

        log::info!("Executing validated command using TokioCommand: {} with args {:?}", program, args);

        let mut command = TokioCommand::new(program);
        command.args(args);
        command.stdout(std::process::Stdio::piped());
        command.stderr(std::process::Stdio::piped());

        match command.spawn() {
            Ok(mut child) => {
                let stdout = child.stdout.take().expect("Failed to capture stdout");
                let stderr = child.stderr.take().expect("Failed to capture stderr");

                let mut stdout_reader = BufReader::new(stdout).lines();
                let mut stderr_reader = BufReader::new(stderr).lines();

                loop {
                    tokio::select! {
                        // Read from stdout
                        Ok(Some(line)) = stdout_reader.next_line() => {
                            log::info!("RAW STDOUT BYTES AS STRING: {:?}", line);
                            yield Ok(Event::default().data(line));
                        },
                        Ok(Some(line)) = stderr_reader.next_line() => {
                            log::info!("RAW STDERR BYTES AS STRING: {:?}", line);
                            yield Ok(Event::default().data(line));
                        },
                        else => break,
                    }
                }

                match child.wait().await {
                    Ok(status) => {
                        yield Ok(Event::default().event("done").data(format!("[COMMAND_FINISHED code={:?}]", status.code())));
                    }
                    Err(e) => {
                        yield Ok(Event::default().event("error").data(format!("[ERROR: Failed to wait for command. Error: {}]", e)));
                    }
                }
            },
            Err(e) => {
                yield Ok(Event::default().event("error").data(format!("[ERROR: Failed to spawn command '{}'. Error: {}]", program, e)));
            }
        }
    };

    Sse::new(stream)
}

async fn proxy_handler(
    AxumState(state): AxumState<AppState>,
    method: Method,
    headers: HeaderMap,
    uri: Uri,
    body: Body,
) -> Result<Response, StatusCode> {
    let path = uri.path();
    let query = uri.query().unwrap_or("");

    let target_url = {
        // This whole block will evaluate to a single String value.

        let settings = state.app_handle.state::<AppSettings>();
        let ollama_url_guard = settings.ollama_url.lock().unwrap();
        
        let base_url = ollama_url_guard
            .as_deref()
            .unwrap_or("http://127.0.0.1:11434");

        // 2. This is the last line. With no semicolon, its value is "returned"
        //    from the block and assigned to `target_url`.
        format!("{}{}?{}", base_url, path, query)

    };

    log::info!("Proxying {} request to: {}", method, target_url);

    let body_bytes = match body.collect().await {
        Ok(collected) => collected.to_bytes(),
        Err(e) => {
            log::error!("Failed to collect request body: {}", e);
            return Err(StatusCode::INTERNAL_SERVER_ERROR);
        }
    };

    let reqwest_request = state
        .http_client
        .request(method, &target_url)
        .headers(headers)
        .body(body_bytes);

    match reqwest_request.send().await {
        Ok(upstream_response) => {
            let mut response_builder = Response::builder()
                .status(upstream_response.status())
                .version(upstream_response.version());
            
            if let Some(headers) = response_builder.headers_mut() {
                headers.extend(upstream_response.headers().clone());
            }

            let response_stream = upstream_response.bytes_stream();
            let response_body = Body::from_stream(response_stream);

            Ok(response_builder.body(response_body).unwrap())
        }
        Err(e) => {
            log::error!("Proxy request to Ollama failed: {}", e);
            Err(StatusCode::BAD_GATEWAY)
        }
    }
}


#[derive(Clone)]
struct ServerUrl(String);

#[tauri::command]
fn get_server_url(server_url: State<Mutex<ServerUrl>>) -> String {
    server_url.lock().unwrap().0.clone()
}

#[cfg(not(debug_assertions))]
fn start_static_server(app_handle: tauri::AppHandle) {
    let rt = tokio::runtime::Runtime::new().unwrap();
    rt.block_on(async {
        const SERVER_PORT: u16 = 3838;
        let url = format!("http://127.0.0.1:{}", SERVER_PORT);
        let addr_str = url.replace("http://", "");

        let server_url_state = app_handle.state::<Mutex<ServerUrl>>();
        *server_url_state.lock().unwrap() = ServerUrl(url.clone());

        let resource_path = app_handle
            .path()
            .resource_dir()
            .expect("failed to get resource directory")
            .join("_up_/dist");

        log::info!("Serving static files from: {:?}", resource_path);

        let cors = CorsLayer::new()
            .allow_origin(Any)
            .allow_methods(Any)
            .allow_headers(Any);

        let state = AppState {
            app_handle: app_handle.clone(),
            http_client: Client::new(),
        };

        let app = Router::new()
            .route("/exec", get(exec_handler))
            .route("/v1/*path", any(proxy_handler))
            .route("/api/*path", any(proxy_handler))
            .fallback_service(ServeDir::new(resource_path))
            .with_state(state)
            .layer(cors);

        let listener = tokio::net::TcpListener::bind(&addr_str).await;

        match listener {
            Ok(l) => {
                log::info!("Web server listening on {}", url);
                if let Err(e) = axum::serve(l, app.into_make_service()).await {
                    log::error!("Server error: {}", e);
                }
            }
            Err(e) => {
                log::error!(
                    "FATAL: Failed to bind to address {}. Is another instance running? Error: {}",
                    addr_str,
                    e
                );
            }
        }
    });
}

#[cfg_attr(mobile, tauri::mobile_entry_point)]
pub fn run() {
    tauri::Builder::default()
        .manage(Mutex::new(ServerUrl("".to_string())))
        .manage(AppSettings {
            ollama_url: Mutex::new(None),
        })
        .setup(|app| {
            app.handle().plugin(
                tauri_plugin_log::Builder::default()
                    .level(log::LevelFilter::Info)
                    .build(),
            )?;

            #[cfg(not(debug_assertions))]
            {
                let app_handle = app.handle().clone();
                std::thread::spawn(move || {
                    start_static_server(app_handle);
                });
            }

            #[cfg(debug_assertions)]
            {
                let server_url_state = app.state::<Mutex<ServerUrl>>();
                let dev_url = app.config().build.dev_url.clone().unwrap();
                *server_url_state.lock().unwrap() = ServerUrl(dev_url.to_string());
            }

            let handle = app.handle();
            
            let show = MenuItem::with_id(handle, "show", "Show Launcher", true, None::<&str>)?;
            let quit = MenuItem::with_id(handle, "quit", "Quit", true, None::<&str>)?;
            let menu = Menu::with_items(handle, &[&show, &quit])?;

            let _tray = TrayIconBuilder::new()
                .tooltip("Observer AI is running")
                .icon(app.default_window_icon().cloned().unwrap())
                .menu(&menu)
                .on_menu_event(move |app, event| {
                    // Compare against event.id directly
                    match event.id.as_ref() {
                        "quit" => {
                            log::info!("Exit called");
                            app.exit(0);
                        }
                        "show" => {
                            if let Some(window) = app.get_webview_window("main") {
                                window.show().unwrap();
                                window.set_focus().unwrap();
                            }
                        }
                        _ => {}
                    }
                })
                .build(app)?;

            Ok(())
        })
        .on_window_event(|window, event| match event {
            tauri::WindowEvent::CloseRequested { api, .. } => {
                window.hide().unwrap();
                api.prevent_close();
            }
            _ => {}
        })
        .plugin(tauri_plugin_shell::init())
        .invoke_handler(tauri::generate_handler![
            get_server_url,
            set_ollama_url,
            get_ollama_url,
            check_ollama_servers
        ])
        .run(tauri::generate_context!())
        .expect("error while running tauri application");
}
