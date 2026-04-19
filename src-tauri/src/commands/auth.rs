use serde_json::Value;
use std::io::{ErrorKind, Read, Write};
use std::net::TcpListener;
use std::process::Command;
use std::thread;
use std::time::{Duration, SystemTime, UNIX_EPOCH};
use tauri::{AppHandle, Emitter, Manager, State};

use crate::account::{AccountRepository, AuthType, UpsertSyncedAccountInput};
use crate::auth::{
    self, AuthService, OAuthCallbackFinishedEvent, OAuthLoginResult, PendingOAuthLogin,
};
use crate::error::AppError;
use crate::local_sync::LocalAuthSyncService;
use crate::{AppState, OAuthCallbackListenerHandle};

const OAUTH_CALLBACK_FINISHED_EVENT: &str = "oauth-callback-finished";

#[tauri::command]
pub async fn refresh_token(
    state: State<'_, AppState>,
    account_id: String,
) -> Result<Value, AppError> {
    let (account, credential) = {
        let db = state.db.lock().await;
        let repo = AccountRepository::new(&db);
        (
            repo.get_by_id(&account_id)?,
            repo.get_credential(&account_id)?,
        )
    };

    let auth_service = AuthService::new();
    if account.auth_type == AuthType::OAuthToken {
        let refresh_result = auth_service.refresh_oauth_credential(&credential).await?;
        {
            let db = state.db.lock().await;
            let repo = AccountRepository::new(&db);
            repo.update_credential(
                &account_id,
                &refresh_result.credential_value,
                Some("oauth_json"),
            )?;
            if account.is_default {
                LocalAuthSyncService::write_account_to_default_auth_file(&repo, &account_id)?;
            }
        }

        let mut result = auth_service
            .validate_credential(&account, &refresh_result.credential_value)
            .await?;
        if matches!(&result.status, auth::AuthStatus::Valid) {
            result.message = Some(format!(
                "Token 已刷新并验证有效，刷新时间 {}",
                refresh_result.refreshed_at
            ));
        }
        return Ok(serde_json::to_value(result)?);
    }

    let result = auth_service
        .validate_credential(&account, &credential)
        .await?;
    Ok(serde_json::to_value(result)?)
}

#[tauri::command]
pub async fn validate_token(
    state: State<'_, AppState>,
    account_id: String,
) -> Result<Value, AppError> {
    let (account, credential) = {
        let db = state.db.lock().await;
        let repo = AccountRepository::new(&db);
        (
            repo.get_by_id(&account_id)?,
            repo.get_credential(&account_id)?,
        )
    };

    let auth_service = AuthService::new();
    let result = auth_service
        .validate_credential(&account, &credential)
        .await?;
    Ok(serde_json::to_value(result)?)
}

#[tauri::command]
pub async fn get_auth_status(
    state: State<'_, AppState>,
    account_id: String,
) -> Result<Value, AppError> {
    let db = state.db.lock().await;
    let repo = AccountRepository::new(&db);
    let account = repo.get_by_id(&account_id)?;
    Ok(serde_json::to_value(&account.status)?)
}

#[tauri::command]
pub async fn prepare_oauth_login(
    app: AppHandle,
    state: State<'_, AppState>,
) -> Result<Value, AppError> {
    let _oauth_guard = state.oauth_flow_lock.lock().await;
    stop_oauth_callback_listener(state.inner()).await;

    let (listener, redirect_port) = bind_oauth_callback_listener(auth::oauth_redirect_port())?;
    let (pending, prepared) = auth::prepare_oauth_login(redirect_port)?;
    {
        let mut pending_guard = state.pending_oauth_login.lock().await;
        *pending_guard = Some(pending.clone());
    }

    if let Err(error) = start_oauth_callback_listener(&app, state.inner(), listener, &pending).await
    {
        let mut pending_guard = state.pending_oauth_login.lock().await;
        *pending_guard = None;
        return Err(error);
    }

    Ok(serde_json::to_value(prepared)?)
}

#[tauri::command]
pub async fn open_oauth_login_url(url: String) -> Result<(), AppError> {
    let parsed = reqwest::Url::parse(&url)
        .map_err(|error| AppError::InvalidInput(format!("授权链接格式无效: {error}")))?;
    if parsed.scheme() != "https" || parsed.host_str() != Some("auth.openai.com") {
        return Err(AppError::InvalidInput(
            "仅允许打开 OpenAI OAuth 授权链接".to_string(),
        ));
    }

    open_url_with_system_browser(&url)
}

#[tauri::command]
pub async fn complete_oauth_callback_login(
    app: AppHandle,
    state: State<'_, AppState>,
    callback_url: String,
) -> Result<Value, AppError> {
    let _oauth_guard = state.oauth_flow_lock.lock().await;
    let pending = current_pending_oauth_login(state.inner()).await?;
    let result = complete_oauth_login_internal(&app, state.inner(), &callback_url).await?;
    clear_pending_oauth_if_matches(state.inner(), &pending.state).await;
    stop_oauth_callback_listener(state.inner()).await;
    Ok(serde_json::to_value(result)?)
}

#[tauri::command]
pub async fn cancel_oauth_login(state: State<'_, AppState>) -> Result<(), AppError> {
    let _oauth_guard = state.oauth_flow_lock.lock().await;
    {
        let mut pending_guard = state.pending_oauth_login.lock().await;
        *pending_guard = None;
    }
    stop_oauth_callback_listener(state.inner()).await;
    Ok(())
}

async fn current_pending_oauth_login(state: &AppState) -> Result<PendingOAuthLogin, AppError> {
    let pending_guard = state.pending_oauth_login.lock().await;
    pending_guard
        .clone()
        .ok_or_else(|| AppError::InvalidInput("请先打开 OAuth 授权页面".to_string()))
}

async fn complete_oauth_login_internal(
    app: &AppHandle,
    state: &AppState,
    callback_url: &str,
) -> Result<OAuthLoginResult, AppError> {
    let pending = current_pending_oauth_login(state).await?;
    let completed = auth::complete_oauth_callback_login(&pending, callback_url).await?;
    let credential_value = serde_json::to_string(&completed.auth_json)?;
    let email = completed.email.clone();
    let stable_id = format!("oauth-login-{}", completed.account_id);
    let account_name = auth::resolve_account_display_name(
        completed.name.as_deref(),
        email.as_deref(),
        "OAuth 登录账号",
    );

    let account = {
        let db = state.db.lock().await;
        let repo = AccountRepository::new(&db);
        repo.upsert_synced_account(UpsertSyncedAccountInput {
            stable_id,
            name: account_name,
            auth_type: AuthType::OAuthToken,
            email,
            organization: Some("OAuth 网页登录".to_string()),
            color: Some("#4f8ef7".to_string()),
            credential_value,
            credential_type: Some("oauth_json".to_string()),
            codex_profile: None,
        })?
    };

    let result = OAuthLoginResult {
        account_id: account.id,
        account_name: account.name,
        auth_type: account.auth_type.to_string(),
    };
    let _ = app.emit(
        "account-status-updated",
        serde_json::json!({
            "account_id": result.account_id,
            "status": "unknown",
            "message": "OAuth 登录已导入",
        }),
    );
    Ok(result)
}

async fn stop_oauth_callback_listener(state: &AppState) {
    let listener_handle = {
        let mut listener_guard = state.oauth_listener.lock().await;
        listener_guard.take()
    };

    let Some(mut listener_handle) = listener_handle else {
        return;
    };

    if let Some(shutdown_tx) = listener_handle.shutdown_tx.take() {
        let _ = shutdown_tx.send(());
    }

    if let Some(task) = listener_handle.task.take() {
        let _ = tauri::async_runtime::spawn_blocking(move || {
            let _ = task.join();
        })
        .await;
    }
}

async fn clear_pending_oauth_if_matches(state: &AppState, expected_state: &str) {
    let mut pending_guard = state.pending_oauth_login.lock().await;
    if pending_guard
        .as_ref()
        .is_some_and(|pending| pending.state.as_str() == expected_state)
    {
        *pending_guard = None;
    }
}

async fn emit_oauth_callback_finished(app: &AppHandle, payload: OAuthCallbackFinishedEvent) {
    let _ = app.emit(OAUTH_CALLBACK_FINISHED_EVENT, payload);
}

async fn start_oauth_callback_listener(
    app: &AppHandle,
    state: &AppState,
    listener: TcpListener,
    pending: &PendingOAuthLogin,
) -> Result<(), AppError> {
    listener
        .set_nonblocking(true)
        .map_err(|error| AppError::Other(format!("无法设置 OAuth 回调监听模式: {error}")))?;

    let (shutdown_tx, shutdown_rx) = std::sync::mpsc::channel();
    let app_handle = app.clone();
    let pending_login = pending.clone();
    let task = thread::spawn(move || {
        run_oauth_callback_listener(app_handle, listener, pending_login, shutdown_rx);
    });

    let mut listener_guard = state.oauth_listener.lock().await;
    *listener_guard = Some(OAuthCallbackListenerHandle {
        shutdown_tx: Some(shutdown_tx),
        task: Some(task),
    });
    Ok(())
}

fn run_oauth_callback_listener(
    app: AppHandle,
    listener: TcpListener,
    pending: PendingOAuthLogin,
    shutdown_rx: std::sync::mpsc::Receiver<()>,
) {
    loop {
        if shutdown_rx.try_recv().is_ok() {
            break;
        }

        if oauth_login_expired(pending.expires_at) {
            tauri::async_runtime::block_on(async {
                let state = app.state::<AppState>();
                clear_pending_oauth_if_matches(state.inner(), &pending.state).await;
                emit_oauth_callback_finished(
                    &app,
                    OAuthCallbackFinishedEvent {
                        result: None,
                        error: Some("OAuth 授权已超时，请重新打开授权页面".to_string()),
                    },
                )
                .await;
            });
            break;
        }

        match listener.accept() {
            Ok((mut stream, _)) => {
                let path = match read_oauth_request_path(&mut stream) {
                    Ok(path) => path,
                    Err(error) => {
                        write_oauth_html_response(
                            &mut stream,
                            "400 Bad Request",
                            "授权失败",
                            &error,
                        );
                        break;
                    }
                };

                if path == "/cancel" {
                    write_oauth_html_response(
                        &mut stream,
                        "200 OK",
                        "授权已取消",
                        "当前授权监听已取消，可以关闭这个页面。",
                    );
                    break;
                }

                if !path.starts_with("/auth/callback") {
                    write_oauth_html_response(
                        &mut stream,
                        "404 Not Found",
                        "未识别的回调地址",
                        "当前地址不是 Codex Manager 的 OAuth 回调地址，可以关闭这个页面。",
                    );
                    continue;
                }

                let callback_url = match build_oauth_callback_url(&pending.redirect_uri, &path) {
                    Ok(callback_url) => callback_url,
                    Err(error) => {
                        write_oauth_html_response(
                            &mut stream,
                            "400 Bad Request",
                            "授权失败",
                            &error,
                        );
                        break;
                    }
                };
                let callback_result = tauri::async_runtime::block_on(async {
                    let state = app.state::<AppState>();
                    let pending_matches = {
                        let pending_guard = state.pending_oauth_login.lock().await;
                        pending_guard
                            .as_ref()
                            .is_some_and(|current| current.state.as_str() == pending.state.as_str())
                    };
                    if !pending_matches {
                        return Err(AppError::AuthFailed(
                            "当前授权会话已失效，请回到应用重新打开授权页面。".to_string(),
                        ));
                    }

                    let result =
                        complete_oauth_login_internal(&app, state.inner(), &callback_url).await;
                    clear_pending_oauth_if_matches(state.inner(), &pending.state).await;
                    result
                });

                match callback_result {
                    Ok(result) => {
                        write_oauth_html_response(
                            &mut stream,
                            "200 OK",
                            "授权完成",
                            "账号已经写入 Codex Manager，可以回到应用继续操作。",
                        );
                        restore_main_window(&app);
                        tauri::async_runtime::block_on(async {
                            emit_oauth_callback_finished(
                                &app,
                                OAuthCallbackFinishedEvent {
                                    result: Some(result),
                                    error: None,
                                },
                            )
                            .await;
                        });
                    }
                    Err(error) => {
                        write_oauth_html_response(
                            &mut stream,
                            "400 Bad Request",
                            "授权失败",
                            &error.to_string(),
                        );
                        restore_main_window(&app);
                        tauri::async_runtime::block_on(async {
                            emit_oauth_callback_finished(
                                &app,
                                OAuthCallbackFinishedEvent {
                                    result: None,
                                    error: Some(error.to_string()),
                                },
                            )
                            .await;
                        });
                    }
                }
                break;
            }
            Err(error) if error.kind() == ErrorKind::WouldBlock => {
                thread::sleep(Duration::from_millis(120));
            }
            Err(error) => {
                tauri::async_runtime::block_on(async {
                    emit_oauth_callback_finished(
                        &app,
                        OAuthCallbackFinishedEvent {
                            result: None,
                            error: Some(format!("OAuth 回调监听失败: {error}")),
                        },
                    )
                    .await;
                });
                break;
            }
        }
    }

    tauri::async_runtime::block_on(async {
        let state = app.state::<AppState>();
        let mut listener_guard = state.oauth_listener.lock().await;
        *listener_guard = None;
    });
}

fn bind_oauth_callback_listener(preferred_port: u16) -> Result<(TcpListener, u16), AppError> {
    match TcpListener::bind(("127.0.0.1", preferred_port)) {
        Ok(listener) => Ok((listener, preferred_port)),
        Err(error) if error.kind() == ErrorKind::AddrInUse => {
            let fallback = TcpListener::bind(("127.0.0.1", 0)).map_err(|fallback_error| {
                AppError::Other(format!(
                    "无法启动 OAuth 回调监听 127.0.0.1:{preferred_port}: {error}；自动回退到空闲端口也失败: {fallback_error}"
                ))
            })?;
            let port = fallback
                .local_addr()
                .map_err(|error| AppError::Other(format!("无法读取 OAuth 回调监听端口: {error}")))?
                .port();
            Ok((fallback, port))
        }
        Err(error) => Err(AppError::Other(format!(
            "无法启动 OAuth 回调监听 127.0.0.1:{preferred_port}: {error}"
        ))),
    }
}

fn read_oauth_request_path(stream: &mut std::net::TcpStream) -> Result<String, String> {
    stream
        .set_read_timeout(Some(Duration::from_secs(4)))
        .map_err(|error| format!("设置 OAuth 回调读取超时失败: {error}"))?;
    let mut buffer = [0_u8; 8192];
    let bytes_read = stream
        .read(&mut buffer)
        .map_err(|error| format!("读取 OAuth 回调请求失败: {error}"))?;
    if bytes_read == 0 {
        return Err("OAuth 回调连接已关闭".to_string());
    }

    let request = String::from_utf8_lossy(&buffer[..bytes_read]);
    let request_line = request
        .lines()
        .next()
        .ok_or_else(|| "OAuth 回调请求为空".to_string())?;
    let mut parts = request_line.split_whitespace();
    let method = parts.next().unwrap_or_default();
    if method != "GET" {
        return Err(format!("不支持的 OAuth 回调请求方法: {method}"));
    }

    parts
        .next()
        .map(ToString::to_string)
        .ok_or_else(|| "OAuth 回调请求缺少路径".to_string())
}

fn build_oauth_callback_url(redirect_uri: &str, path: &str) -> Result<String, String> {
    let mut callback_url = reqwest::Url::parse(redirect_uri)
        .map_err(|error| format!("OAuth redirect_uri 无效: {error}"))?;
    let request_url = reqwest::Url::parse(&format!("http://localhost{path}"))
        .map_err(|error| format!("OAuth 回调路径无效: {error}"))?;
    callback_url.set_path(request_url.path());
    callback_url.set_query(request_url.query());
    callback_url.set_fragment(request_url.fragment());
    Ok(callback_url.to_string())
}

fn oauth_login_expired(expires_at: i64) -> bool {
    SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .map(|duration| duration.as_secs() as i64 >= expires_at)
        .unwrap_or(true)
}

fn write_oauth_html_response(
    stream: &mut std::net::TcpStream,
    status_line: &str,
    title: &str,
    detail: &str,
) {
    let body = format!(
        "<!doctype html><html lang=\"zh-CN\"><head><meta charset=\"utf-8\"><meta name=\"viewport\" content=\"width=device-width, initial-scale=1\"><title>{}</title><style>body{{margin:0;padding:32px;font-family:-apple-system,BlinkMacSystemFont,'Segoe UI',sans-serif;background:#f4f7fb;color:#152033}}main{{max-width:560px;margin:0 auto;padding:24px;border-radius:20px;background:#fff;box-shadow:0 14px 34px rgba(21,32,51,.08)}}h1{{margin:0 0 10px;font-size:24px}}p{{margin:0;color:#52627b;line-height:1.6;word-break:break-word}}</style></head><body><main><h1>{}</h1><p>{}</p></main></body></html>",
        escape_html(title),
        escape_html(title),
        escape_html(detail)
    );
    let response = format!(
        "HTTP/1.1 {status_line}\r\nContent-Type: text/html; charset=utf-8\r\nCache-Control: no-store\r\nContent-Length: {}\r\nConnection: close\r\n\r\n{}",
        body.as_bytes().len(),
        body
    );
    let _ = stream.write_all(response.as_bytes());
    let _ = stream.flush();
}

fn escape_html(input: &str) -> String {
    input
        .replace('&', "&amp;")
        .replace('<', "&lt;")
        .replace('>', "&gt;")
        .replace('"', "&quot;")
        .replace('\'', "&#39;")
}

fn restore_main_window(app: &AppHandle) {
    if let Some(window) = app.get_webview_window("main") {
        let _ = window.show();
        let _ = window.set_focus();
    }
}

fn open_url_with_system_browser(url: &str) -> Result<(), AppError> {
    #[cfg(target_os = "windows")]
    {
        Command::new("rundll32.exe")
            .args(["url.dll,FileProtocolHandler", url])
            .spawn()
            .or_else(|primary_error| {
                Command::new("explorer.exe")
                    .arg(url)
                    .spawn()
                    .map_err(|fallback_error| {
                        std::io::Error::new(
                            fallback_error.kind(),
                            format!(
                                "rundll32 启动失败: {primary_error}; explorer 启动失败: {fallback_error}"
                            ),
                        )
                    })
            })?;
        Ok(())
    }

    #[cfg(target_os = "macos")]
    {
        Command::new("open").arg(url).spawn()?;
        Ok(())
    }

    #[cfg(all(unix, not(target_os = "macos")))]
    {
        Command::new("xdg-open").arg(url).spawn()?;
        Ok(())
    }
}
