use crate::error::{AppError, AppResult};
use crate::usage::ApiUsageEventRecord;
use chrono::Utc;
use serde::{Deserialize, Serialize};
use serde_json::{Map, Value};
use std::collections::{BTreeMap, BTreeSet};
use std::env;
use std::path::{Path, PathBuf};
use std::process::Stdio;
use tokio::io::{AsyncBufReadExt, AsyncRead, BufReader};
use tokio::process::Command;
use toml::Value as TomlValue;
use uuid::Uuid;

#[derive(Debug, Deserialize)]
pub struct CodexExecInput {
    pub account_id: String,
    pub prompt: String,
    pub working_directory: Option<String>,
    pub model: Option<String>,
    pub profile: Option<String>,
    pub sandbox: Option<String>,
    pub config_overrides: Option<Vec<String>>,
    pub skip_git_repo_check: Option<bool>,
}

#[derive(Debug, Deserialize)]
pub struct CodexInteractiveInput {
    pub account_id: String,
    pub prompt: Option<String>,
    pub working_directory: Option<String>,
    pub model: Option<String>,
    pub profile: Option<String>,
    pub sandbox: Option<String>,
}

#[derive(Debug, Deserialize)]
pub struct CodexCliLaunchInput {
    pub account_id: Option<String>,
    pub working_directory: Option<String>,
    pub model: Option<String>,
}

#[derive(Debug, Deserialize)]
pub struct CodexAppLaunchInput {
    pub account_id: Option<String>,
}

#[derive(Debug, Serialize)]
pub struct CodexModelOption {
    pub label: String,
    pub value: String,
}

#[derive(Debug, Serialize)]
pub struct CodexLauncherConfig {
    pub default_model: Option<String>,
    pub model_options: Vec<CodexModelOption>,
}

#[derive(Debug, Serialize)]
pub struct CodexLaunchResult {
    pub session_id: String,
    pub status: String,
    pub exit_code: Option<i32>,
    pub usage_event_count: usize,
    pub message: String,
    pub stderr_preview: Option<String>,
}

#[derive(Debug)]
pub struct CodexExecOutcome {
    pub exit_code: Option<i32>,
    pub usage_events: Vec<ApiUsageEventRecord>,
    pub stderr_preview: Option<String>,
    pub message: String,
}

#[derive(Debug, Clone)]
pub struct CodexCommandTarget {
    executable: PathBuf,
    runner: CommandRunner,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum CommandRunner {
    Direct,
    PowerShellScript,
    CommandScript,
}

#[derive(Debug)]
struct JsonlUsageReadResult {
    events: Vec<ParsedUsageEvent>,
    json_line_count: usize,
    ignored_line_count: usize,
}

#[derive(Debug, Clone)]
struct ParsedUsageEvent {
    model: Option<String>,
    response_id: Option<String>,
    request_id: Option<String>,
    input_tokens: i64,
    output_tokens: i64,
    total_tokens: i64,
    cached_input_tokens: Option<i64>,
    reasoning_tokens: Option<i64>,
    raw_usage_json: String,
}

impl CodexCommandTarget {
    pub fn discover() -> AppResult<Self> {
        if let Some(configured_path) = env::var_os("CODEX_EXECUTABLE") {
            let configured = PathBuf::from(configured_path);
            if configured.exists() {
                return Ok(Self::from_path(configured));
            }
        }

        for executable_name in ["codex.exe", "codex.cmd", "codex.bat", "codex.ps1", "codex"] {
            if let Some(path) = find_in_path(executable_name) {
                return Ok(Self::from_path(path));
            }
        }

        Err(AppError::Other(
            "未找到 codex 命令，请确认 Codex CLI 已安装并位于 PATH 中".to_string(),
        ))
    }

    fn from_path(path: PathBuf) -> Self {
        let runner = match path
            .extension()
            .and_then(|extension| extension.to_str())
            .map(|extension| extension.to_ascii_lowercase())
            .as_deref()
        {
            Some("ps1") => CommandRunner::PowerShellScript,
            Some("cmd") | Some("bat") => CommandRunner::CommandScript,
            _ => CommandRunner::Direct,
        };

        Self {
            executable: path,
            runner,
        }
    }

    pub fn executable_label(&self) -> String {
        self.executable.to_string_lossy().to_string()
    }

    fn build_tokio_command(&self, args: &[String]) -> AppResult<Command> {
        let mut command = match self.runner {
            CommandRunner::Direct => {
                let mut command = Command::new(&self.executable);
                command.args(args);
                command
            }
            CommandRunner::PowerShellScript => {
                let shell = find_powershell()?;
                let mut command = Command::new(shell);
                command
                    .arg("-NoProfile")
                    .arg("-ExecutionPolicy")
                    .arg("Bypass")
                    .arg("-File")
                    .arg(&self.executable)
                    .args(args);
                command
            }
            CommandRunner::CommandScript => {
                let mut command = Command::new("cmd");
                command.arg("/C").arg(&self.executable).args(args);
                command
            }
        };

        command.stdout(Stdio::piped()).stderr(Stdio::piped());
        Ok(command)
    }

    fn build_interactive_command(&self, args: &[String]) -> AppResult<std::process::Command> {
        let mut command = std::process::Command::new("cmd");
        // start 的第一个带引号参数会被解释为窗口标题；使用空标题可以避免
        // Windows 把“Codex”标题误当作需要启动的文件路径。
        command.arg("/C").arg("start").arg("");

        match self.runner {
            CommandRunner::Direct => {
                command.arg(&self.executable);
            }
            CommandRunner::PowerShellScript => {
                let shell = find_powershell()?;
                command
                    .arg(shell)
                    .arg("-NoExit")
                    .arg("-NoProfile")
                    .arg("-ExecutionPolicy")
                    .arg("Bypass")
                    .arg("-File")
                    .arg(&self.executable);
            }
            CommandRunner::CommandScript => {
                command.arg(&self.executable);
            }
        }

        command.args(args);
        Ok(command)
    }
}

pub async fn run_codex_exec(
    target: &CodexCommandTarget,
    input: &CodexExecInput,
    session_id: &str,
    started_at: &str,
) -> AppResult<CodexExecOutcome> {
    let prompt = input.prompt.trim();
    if prompt.is_empty() {
        return Err(AppError::InvalidInput(
            "请输入要交给 Codex 执行的任务".to_string(),
        ));
    }

    let mut args = vec![
        "exec".to_string(),
        "--json".to_string(),
        "--color".to_string(),
        "never".to_string(),
    ];
    append_common_codex_args(&mut args, CodexCommonOptions::from_exec_input(input));
    if input.skip_git_repo_check.unwrap_or(false) {
        args.push("--skip-git-repo-check".to_string());
    }
    args.push(prompt.to_string());

    let mut command = target.build_tokio_command(&args)?;
    if let Some(working_directory) =
        normalize_existing_directory(input.working_directory.as_deref())?
    {
        command.current_dir(working_directory);
    }

    let mut child = command.spawn()?;
    let stdout = child
        .stdout
        .take()
        .ok_or_else(|| AppError::Other("无法读取 Codex stdout".to_string()))?;
    let stderr = child
        .stderr
        .take()
        .ok_or_else(|| AppError::Other("无法读取 Codex stderr".to_string()))?;

    let stdout_task = tokio::spawn(read_jsonl_usage(stdout));
    let stderr_task = tokio::spawn(read_limited_text(stderr, 16_000));
    let status = child.wait().await?;
    let stdout_result = stdout_task
        .await
        .map_err(|error| AppError::Other(format!("解析 Codex stdout 失败: {error}")))??;
    let stderr_preview = stderr_task
        .await
        .map_err(|error| AppError::Other(format!("读取 Codex stderr 失败: {error}")))??;

    let completed_at = Utc::now().to_rfc3339();
    let usage_events = build_usage_records(
        &input.account_id,
        session_id,
        started_at,
        &completed_at,
        stdout_result.events,
    );
    let message = if usage_events.is_empty() {
        if stdout_result.json_line_count == 0 {
            "Codex 已结束，但 stdout 中没有 JSONL 事件".to_string()
        } else {
            "Codex 已结束，但没有识别到 usage 字段".to_string()
        }
    } else {
        format!("Codex 已结束，记录 {} 条用量事件", usage_events.len())
    };

    let stderr_preview = normalize_optional_text(Some(stderr_preview));
    let stderr_preview = if status.success() {
        stderr_preview
    } else {
        stderr_preview.or_else(|| Some("Codex 进程返回非零退出码".to_string()))
    };

    let ignored_message = if stdout_result.ignored_line_count > 0 && usage_events.is_empty() {
        format!(
            "{message}；忽略 {} 行非 JSON 输出",
            stdout_result.ignored_line_count
        )
    } else {
        message
    };

    Ok(CodexExecOutcome {
        exit_code: status.code(),
        usage_events,
        stderr_preview,
        message: ignored_message,
    })
}

pub fn open_interactive_codex(
    target: &CodexCommandTarget,
    input: &CodexInteractiveInput,
) -> AppResult<()> {
    let mut args = Vec::new();
    append_common_codex_args(&mut args, CodexCommonOptions::from_interactive_input(input));
    if let Some(prompt) = normalize_optional_text(input.prompt.clone()) {
        args.push(prompt);
    }

    let mut command = target.build_interactive_command(&args)?;
    if let Some(working_directory) =
        normalize_existing_directory(input.working_directory.as_deref())?
    {
        command.current_dir(working_directory);
    }

    command.spawn()?;
    Ok(())
}

pub fn open_codex_cli_terminal(
    target: &CodexCommandTarget,
    input: &CodexCliLaunchInput,
) -> AppResult<()> {
    let mut args = Vec::new();
    append_common_codex_args(&mut args, CodexCommonOptions::from_cli_launch_input(input));

    let mut command = target.build_interactive_command(&args)?;
    if let Some(working_directory) =
        normalize_existing_directory(input.working_directory.as_deref())?
    {
        command.current_dir(working_directory);
    }

    command.spawn()?;
    Ok(())
}

pub fn open_codex_desktop_app() -> AppResult<()> {
    #[cfg(target_os = "windows")]
    {
        let mut command = std::process::Command::new("explorer.exe");
        command
            .arg(r"shell:AppsFolder\OpenAI.Codex_2p2nqsd0c76g0!App")
            .stdin(Stdio::null())
            .stdout(Stdio::null())
            .stderr(Stdio::null());
        command.spawn()?;
        Ok(())
    }

    #[cfg(not(target_os = "windows"))]
    {
        Err(AppError::Other(
            "当前平台暂不支持启动 Codex App".to_string(),
        ))
    }
}

pub fn read_codex_launcher_config() -> AppResult<CodexLauncherConfig> {
    let codex_home = resolve_codex_home()?;
    let default_model = read_default_model(&codex_home.join("config.toml"))?;
    let mut model_options = read_model_options(&codex_home.join("models_cache.json"))?;

    if let Some(default_model_value) = default_model.as_deref() {
        ensure_model_option(&mut model_options, default_model_value);
    }

    Ok(CodexLauncherConfig {
        default_model,
        model_options,
    })
}

pub fn prompt_preview(value: Option<&str>) -> Option<String> {
    value
        .map(str::trim)
        .filter(|text| !text.is_empty())
        .map(|text| {
            let mut preview = text.chars().take(80).collect::<String>();
            if text.chars().count() > 80 {
                preview.push_str("...");
            }
            preview
        })
}

struct CodexCommonOptions<'a> {
    working_directory: Option<&'a str>,
    model: Option<&'a str>,
    profile: Option<&'a str>,
    sandbox: Option<&'a str>,
    config_overrides: &'a [String],
}

impl<'a> CodexCommonOptions<'a> {
    fn from_exec_input(input: &'a CodexExecInput) -> Self {
        Self {
            working_directory: input.working_directory.as_deref(),
            model: input.model.as_deref(),
            profile: input.profile.as_deref(),
            sandbox: input.sandbox.as_deref(),
            config_overrides: input.config_overrides.as_deref().unwrap_or(&[]),
        }
    }

    fn from_interactive_input(input: &'a CodexInteractiveInput) -> Self {
        Self {
            working_directory: input.working_directory.as_deref(),
            model: input.model.as_deref(),
            profile: input.profile.as_deref(),
            sandbox: input.sandbox.as_deref(),
            config_overrides: &[],
        }
    }

    fn from_cli_launch_input(input: &'a CodexCliLaunchInput) -> Self {
        Self {
            working_directory: input.working_directory.as_deref(),
            model: input.model.as_deref(),
            profile: None,
            sandbox: None,
            config_overrides: &[],
        }
    }
}

fn append_common_codex_args(args: &mut Vec<String>, options: CodexCommonOptions<'_>) {
    if let Some(working_directory) = normalize_text(options.working_directory) {
        args.push("-C".to_string());
        args.push(working_directory);
    }

    if let Some(model) = normalize_text(options.model) {
        args.push("-m".to_string());
        args.push(model);
    }

    if let Some(profile) = normalize_text(options.profile) {
        args.push("-p".to_string());
        args.push(profile);
    }

    if let Some(sandbox) = normalize_text(options.sandbox) {
        args.push("-s".to_string());
        args.push(sandbox);
    }

    for config_override in options.config_overrides {
        if let Some(config_override) = normalize_text(Some(config_override)) {
            args.push("-c".to_string());
            args.push(config_override);
        }
    }
}

async fn read_jsonl_usage<R>(reader: R) -> AppResult<JsonlUsageReadResult>
where
    R: AsyncRead + Unpin,
{
    let mut lines = BufReader::new(reader).lines();
    let mut deduped = BTreeMap::<String, ParsedUsageEvent>::new();
    let mut json_line_count = 0;
    let mut ignored_line_count = 0;

    while let Some(line) = lines.next_line().await? {
        let trimmed = line.trim();
        if trimmed.is_empty() {
            continue;
        }

        let Ok(value) = serde_json::from_str::<Value>(trimmed) else {
            ignored_line_count += 1;
            continue;
        };
        json_line_count += 1;

        let mut candidates = Vec::new();
        collect_usage_candidates(&value, &value, &mut candidates);
        for candidate in candidates {
            let key = candidate
                .response_id
                .clone()
                .or_else(|| candidate.request_id.clone())
                .unwrap_or_else(|| "session-final-usage".to_string());
            deduped.insert(key, candidate);
        }
    }

    Ok(JsonlUsageReadResult {
        events: deduped.into_values().collect(),
        json_line_count,
        ignored_line_count,
    })
}

async fn read_limited_text<R>(reader: R, max_chars: usize) -> AppResult<String>
where
    R: AsyncRead + Unpin,
{
    let mut lines = BufReader::new(reader).lines();
    let mut output = String::new();

    while let Some(line) = lines.next_line().await? {
        if output.chars().count() >= max_chars {
            break;
        }
        output.push_str(&line);
        output.push('\n');
    }

    Ok(output)
}

fn collect_usage_candidates(root: &Value, value: &Value, candidates: &mut Vec<ParsedUsageEvent>) {
    match value {
        Value::Object(object) => {
            if let Some(usage) = object.get("usage") {
                if let Some(candidate) = parse_usage_candidate(root, usage) {
                    candidates.push(candidate);
                }
            }

            if let Some(candidate) = parse_usage_candidate(root, value) {
                candidates.push(candidate);
            }

            for (key, child) in object {
                if key == "usage" {
                    continue;
                }
                collect_usage_candidates(root, child, candidates);
            }
        }
        Value::Array(items) => {
            for item in items {
                collect_usage_candidates(root, item, candidates);
            }
        }
        _ => {}
    }
}

fn parse_usage_candidate(root: &Value, usage: &Value) -> Option<ParsedUsageEvent> {
    let object = usage.as_object()?;
    let input_tokens = read_i64(object, &["input_tokens", "prompt_tokens"]).unwrap_or(0);
    let output_tokens = read_i64(object, &["output_tokens", "completion_tokens"]).unwrap_or(0);
    let total_tokens = read_i64(object, &["total_tokens"])
        .unwrap_or_else(|| input_tokens.saturating_add(output_tokens));

    if input_tokens == 0 && output_tokens == 0 && total_tokens == 0 {
        return None;
    }

    let cached_input_tokens = nested_i64(
        usage,
        &[
            &["input_tokens_details", "cached_tokens"],
            &["prompt_tokens_details", "cached_tokens"],
            &["cached_input_tokens"],
            &["input_cached_tokens"],
        ],
    );
    let reasoning_tokens = nested_i64(
        usage,
        &[
            &["output_tokens_details", "reasoning_tokens"],
            &["completion_tokens_details", "reasoning_tokens"],
            &["reasoning_tokens"],
        ],
    );

    Some(ParsedUsageEvent {
        model: find_string(root, &["model", "model_name"]),
        response_id: find_string(root, &["response_id", "responseId"]),
        request_id: find_string(root, &["request_id", "requestId", "x_request_id"]),
        input_tokens,
        output_tokens,
        total_tokens,
        cached_input_tokens,
        reasoning_tokens,
        raw_usage_json: serde_json::to_string(usage).unwrap_or_else(|_| "{}".to_string()),
    })
}

fn build_usage_records(
    account_id: &str,
    session_id: &str,
    started_at: &str,
    completed_at: &str,
    events: Vec<ParsedUsageEvent>,
) -> Vec<ApiUsageEventRecord> {
    events
        .into_iter()
        .map(|event| ApiUsageEventRecord {
            id: Uuid::new_v4().to_string(),
            account_id: account_id.to_string(),
            session_id: Some(session_id.to_string()),
            source: "codex_exec_json".to_string(),
            endpoint: Some("codex exec --json".to_string()),
            model: event.model,
            response_id: event.response_id,
            request_id: event.request_id,
            status_code: None,
            input_tokens: event.input_tokens,
            output_tokens: event.output_tokens,
            total_tokens: event.total_tokens,
            cached_input_tokens: event.cached_input_tokens,
            reasoning_tokens: event.reasoning_tokens,
            estimated_cost: 0.0,
            raw_usage_json: Some(event.raw_usage_json),
            is_complete: true,
            error_message: None,
            started_at: started_at.to_string(),
            completed_at: completed_at.to_string(),
            created_at: Utc::now().to_rfc3339(),
        })
        .collect()
}

fn read_i64(object: &Map<String, Value>, keys: &[&str]) -> Option<i64> {
    keys.iter()
        .find_map(|key| object.get(*key))
        .and_then(value_to_i64)
}

fn nested_i64(value: &Value, paths: &[&[&str]]) -> Option<i64> {
    paths.iter().find_map(|path| {
        let mut current = value;
        for key in *path {
            current = current.get(*key)?;
        }
        value_to_i64(current)
    })
}

fn value_to_i64(value: &Value) -> Option<i64> {
    value
        .as_i64()
        .or_else(|| value.as_u64().and_then(|number| i64::try_from(number).ok()))
        .or_else(|| value.as_f64().map(|number| number.round() as i64))
}

fn find_string(value: &Value, keys: &[&str]) -> Option<String> {
    match value {
        Value::Object(object) => {
            for key in keys {
                if let Some(text) = object.get(*key).and_then(Value::as_str) {
                    let trimmed = text.trim();
                    if !trimmed.is_empty() {
                        return Some(trimmed.to_string());
                    }
                }
            }

            for child in object.values() {
                if let Some(text) = find_string(child, keys) {
                    return Some(text);
                }
            }
            None
        }
        Value::Array(items) => items.iter().find_map(|item| find_string(item, keys)),
        _ => None,
    }
}

fn find_in_path(executable_name: &str) -> Option<PathBuf> {
    let path_variable = env::var_os("PATH")?;
    env::split_paths(&path_variable)
        .map(|directory| directory.join(executable_name))
        .find(|candidate| candidate.exists())
}

fn find_powershell() -> AppResult<PathBuf> {
    find_in_path("pwsh.exe")
        .or_else(|| find_in_path("powershell.exe"))
        .ok_or_else(|| AppError::Other("未找到 PowerShell，无法启动 codex.ps1".to_string()))
}

pub fn resolve_codex_home() -> AppResult<PathBuf> {
    if let Ok(codex_home) = env::var("CODEX_HOME") {
        if let Some(path) = normalize_text(Some(codex_home.as_str())) {
            return Ok(PathBuf::from(path));
        }
    }

    if let Ok(user_profile) = env::var("USERPROFILE") {
        if let Some(path) = normalize_text(Some(user_profile.as_str())) {
            return Ok(PathBuf::from(path).join(".codex"));
        }
    }

    if let Ok(home) = env::var("HOME") {
        if let Some(path) = normalize_text(Some(home.as_str())) {
            return Ok(PathBuf::from(path).join(".codex"));
        }
    }

    Err(AppError::InvalidInput(
        "无法推断 Codex 配置目录，请确认 CODEX_HOME 或 USERPROFILE 环境变量可用".to_string(),
    ))
}

fn read_default_model(config_path: &Path) -> AppResult<Option<String>> {
    if !config_path.exists() {
        return Ok(None);
    }

    let raw_text = std::fs::read_to_string(config_path)?;
    let config_value: TomlValue = toml::from_str(&raw_text)
        .map_err(|error| AppError::Other(format!("读取 Codex config.toml 失败: {error}")))?;

    Ok(config_value
        .get("model")
        .and_then(TomlValue::as_str)
        .map(str::trim)
        .filter(|text| !text.is_empty())
        .map(ToString::to_string))
}

fn read_model_options(models_cache_path: &Path) -> AppResult<Vec<CodexModelOption>> {
    if !models_cache_path.exists() {
        return Ok(Vec::new());
    }

    let raw_text = std::fs::read_to_string(models_cache_path)?;
    let cache_value: Value = serde_json::from_str(&raw_text)?;
    let mut model_options = Vec::new();
    let mut seen_values = BTreeSet::new();

    if let Some(models) = cache_value.get("models").and_then(Value::as_array) {
        for model in models {
            let Some(value) = model
                .get("slug")
                .and_then(Value::as_str)
                .map(str::trim)
                .filter(|text| !text.is_empty())
            else {
                continue;
            };

            if !seen_values.insert(value.to_string()) {
                continue;
            }

            let label = model
                .get("display_name")
                .and_then(Value::as_str)
                .map(str::trim)
                .filter(|text| !text.is_empty())
                .unwrap_or(value)
                .to_string();

            model_options.push(CodexModelOption {
                label,
                value: value.to_string(),
            });
        }
    }

    Ok(model_options)
}

fn ensure_model_option(model_options: &mut Vec<CodexModelOption>, value: &str) {
    if model_options.iter().any(|option| option.value == value) {
        return;
    }

    model_options.insert(
        0,
        CodexModelOption {
            label: value.to_string(),
            value: value.to_string(),
        },
    );
}

fn normalize_existing_directory(value: Option<&str>) -> AppResult<Option<PathBuf>> {
    let Some(text) = normalize_text(value) else {
        return Ok(None);
    };

    let path = Path::new(&text);
    if !path.exists() || !path.is_dir() {
        return Err(AppError::InvalidInput(format!(
            "工作目录不存在或不是文件夹: {text}"
        )));
    }

    Ok(Some(path.to_path_buf()))
}

fn normalize_text(value: Option<&str>) -> Option<String> {
    value
        .map(str::trim)
        .filter(|text| !text.is_empty())
        .map(ToString::to_string)
}

fn normalize_optional_text(value: Option<String>) -> Option<String> {
    value
        .map(|text| text.trim().to_string())
        .filter(|text| !text.is_empty())
}

#[cfg(test)]
mod tests {
    use super::*;
    use serde_json::json;
    use std::time::{SystemTime, UNIX_EPOCH};

    #[test]
    fn parses_chat_completion_usage() {
        let event = json!({
            "model": "gpt-4o",
            "response_id": "chatcmpl_123",
            "usage": {
                "prompt_tokens": 10,
                "completion_tokens": 5,
                "total_tokens": 15,
                "prompt_tokens_details": {
                    "cached_tokens": 2
                },
                "completion_tokens_details": {
                    "reasoning_tokens": 1
                }
            }
        });
        let mut candidates = Vec::new();

        collect_usage_candidates(&event, &event, &mut candidates);

        assert_eq!(candidates.len(), 1);
        assert_eq!(candidates[0].model.as_deref(), Some("gpt-4o"));
        assert_eq!(candidates[0].response_id.as_deref(), Some("chatcmpl_123"));
        assert_eq!(candidates[0].input_tokens, 10);
        assert_eq!(candidates[0].output_tokens, 5);
        assert_eq!(candidates[0].total_tokens, 15);
        assert_eq!(candidates[0].cached_input_tokens, Some(2));
        assert_eq!(candidates[0].reasoning_tokens, Some(1));
    }

    #[test]
    fn parses_responses_usage() {
        let event = json!({
            "model": "gpt-5.4",
            "request_id": "req_123",
            "payload": {
                "usage": {
                    "input_tokens": 20,
                    "output_tokens": 8,
                    "total_tokens": 28,
                    "input_tokens_details": {
                        "cached_tokens": 4
                    },
                    "output_tokens_details": {
                        "reasoning_tokens": 3
                    }
                }
            }
        });
        let mut candidates = Vec::new();

        collect_usage_candidates(&event, &event, &mut candidates);

        assert_eq!(candidates.len(), 1);
        assert_eq!(candidates[0].model.as_deref(), Some("gpt-5.4"));
        assert_eq!(candidates[0].request_id.as_deref(), Some("req_123"));
        assert_eq!(candidates[0].input_tokens, 20);
        assert_eq!(candidates[0].output_tokens, 8);
        assert_eq!(candidates[0].total_tokens, 28);
        assert_eq!(candidates[0].cached_input_tokens, Some(4));
        assert_eq!(candidates[0].reasoning_tokens, Some(3));
    }

    #[test]
    fn reads_default_model_from_config_toml() {
        let temp_dir = unique_temp_dir();
        std::fs::create_dir_all(&temp_dir).unwrap();
        let config_path = temp_dir.join("config.toml");
        std::fs::write(
            &config_path,
            r#"
model = "gpt-5.4"
review_model = "gpt-5.4"
"#,
        )
        .unwrap();

        let default_model = read_default_model(&config_path).unwrap();

        assert_eq!(default_model.as_deref(), Some("gpt-5.4"));
        let _ = std::fs::remove_dir_all(temp_dir);
    }

    #[test]
    fn reads_model_options_from_models_cache() {
        let temp_dir = unique_temp_dir();
        std::fs::create_dir_all(&temp_dir).unwrap();
        let models_cache_path = temp_dir.join("models_cache.json");
        std::fs::write(
            &models_cache_path,
            serde_json::to_string(&json!({
                "models": [
                    {
                        "slug": "gpt-5.4",
                        "display_name": "gpt-5.4"
                    },
                    {
                        "slug": "gpt-5.4-mini",
                        "display_name": "GPT-5.4-Mini"
                    }
                ]
            }))
            .unwrap(),
        )
        .unwrap();

        let model_options = read_model_options(&models_cache_path).unwrap();

        assert_eq!(model_options.len(), 2);
        assert_eq!(model_options[0].value, "gpt-5.4");
        assert_eq!(model_options[1].label, "GPT-5.4-Mini");
        let _ = std::fs::remove_dir_all(temp_dir);
    }

    #[test]
    fn injects_default_model_when_models_cache_missing_it() {
        let mut model_options = vec![CodexModelOption {
            label: "gpt-5.4-mini".to_string(),
            value: "gpt-5.4-mini".to_string(),
        }];

        ensure_model_option(&mut model_options, "gpt-5.4");

        assert_eq!(model_options[0].value, "gpt-5.4");
        assert_eq!(model_options[1].value, "gpt-5.4-mini");
    }

    fn unique_temp_dir() -> PathBuf {
        let suffix = SystemTime::now()
            .duration_since(UNIX_EPOCH)
            .unwrap()
            .as_nanos();
        std::env::temp_dir().join(format!("codex-manager-codex-runtime-tests-{suffix}"))
    }
}
