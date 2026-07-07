use std::cmp::Reverse;
use std::fs;
use std::io::BufRead;
use std::path::PathBuf;
use std::sync::{Mutex, OnceLock};
use std::time::{Duration, Instant};

use crate::codex_transcript::{CodexEvent, parse_line};
use crate::session::CodexSession;

const TRANSCRIPT_INDEX_TTL: Duration = Duration::from_secs(10);

fn sessions_dir() -> PathBuf {
    codex_home().join("sessions")
}

fn codex_home() -> PathBuf {
    std::env::var_os("CODEXCTL_CODEX_HOME")
        .map(PathBuf::from)
        .unwrap_or_else(|| dirs_home().join(".codex"))
}

fn dirs_home() -> PathBuf {
    std::env::var_os("HOME")
        .map(PathBuf::from)
        .unwrap_or_else(|| PathBuf::from("/tmp"))
}

pub fn projects_dir() -> PathBuf {
    sessions_dir()
}

pub fn scan_sessions() -> Vec<CodexSession> {
    let processes = scan_live_codex_processes();
    if processes.is_empty() {
        return Vec::new();
    }

    let transcripts = collect_transcript_summaries();
    sessions_from_live_processes(processes, &transcripts)
}

#[derive(Debug, Clone)]
struct LiveCodexProcess {
    pid: u32,
    cwd: String,
    started_at: u64,
    tty: String,
    cpu_percent: f32,
    mem_mb: f64,
    command_args: String,
}

#[derive(Debug, Clone)]
struct CodexTranscriptSummary {
    session_id: String,
    cwd: String,
    path: PathBuf,
    mtime_ms: u64,
}

struct CachedTranscriptIndex {
    sessions_dir: PathBuf,
    refreshed_at: Instant,
    transcripts: Vec<CodexTranscriptSummary>,
}

static TRANSCRIPT_INDEX_CACHE: OnceLock<Mutex<Option<CachedTranscriptIndex>>> = OnceLock::new();

fn transcript_index_cache() -> &'static Mutex<Option<CachedTranscriptIndex>> {
    TRANSCRIPT_INDEX_CACHE.get_or_init(|| Mutex::new(None))
}

fn scan_live_codex_processes() -> Vec<LiveCodexProcess> {
    if std::env::var_os("CODEXCTL_DISABLE_PROCESS_DISCOVERY").is_some() {
        return Vec::new();
    }

    let output = std::process::Command::new("ps")
        .args(["-eo", "pid=,ppid=,tty=,%cpu=,rss=,etimes=,comm=,args="])
        .env_clear()
        .output();

    let Ok(output) = output else {
        return Vec::new();
    };

    parse_live_codex_processes(&String::from_utf8_lossy(&output.stdout))
}

fn parse_live_codex_processes(ps_stdout: &str) -> Vec<LiveCodexProcess> {
    ps_stdout
        .lines()
        .filter_map(parse_live_codex_process)
        .collect()
}

fn parse_live_codex_process(line: &str) -> Option<LiveCodexProcess> {
    let fields: Vec<&str> = line.split_whitespace().collect();
    if fields.len() < 7 {
        return None;
    }

    let pid = fields[0].parse::<u32>().ok()?;
    let tty = fields[2].to_string();
    let cpu_percent = fields[3].parse::<f32>().unwrap_or(0.0);
    let rss_kb = fields[4].parse::<f64>().unwrap_or(0.0);
    let elapsed_secs = fields[5].parse::<u64>().unwrap_or(0);
    let comm = fields[6];
    let args = fields.get(7..).unwrap_or_default().join(" ");

    if !is_codex_process(comm, &args) {
        return None;
    }

    let cwd = process_cwd(pid)?.to_string_lossy().to_string();
    let command_args = args_after_codex(&args);

    Some(LiveCodexProcess {
        pid,
        cwd,
        started_at: process_started_at_ms(elapsed_secs),
        tty,
        cpu_percent,
        mem_mb: rss_kb / 1024.0,
        command_args,
    })
}

fn is_codex_process(comm: &str, args: &str) -> bool {
    if matches!(comm, "codex" | ".codex-wrapped") {
        return true;
    }

    args.split_whitespace()
        .next()
        .and_then(|arg| PathBuf::from(arg).file_name().map(|name| name.to_owned()))
        .and_then(|name| name.to_str().map(str::to_owned))
        .is_some_and(|name| matches!(name.as_str(), "codex" | ".codex-wrapped"))
}

fn process_cwd(pid: u32) -> Option<PathBuf> {
    #[cfg(target_os = "linux")]
    {
        fs::read_link(format!("/proc/{pid}/cwd")).ok()
    }

    #[cfg(not(target_os = "linux"))]
    {
        let _ = pid;
        None
    }
}

fn process_started_at_ms(elapsed_secs: u64) -> u64 {
    let now_ms = std::time::SystemTime::now()
        .duration_since(std::time::UNIX_EPOCH)
        .unwrap_or_default()
        .as_millis() as u64;
    now_ms.saturating_sub(elapsed_secs.saturating_mul(1000))
}

fn args_after_codex(args: &str) -> String {
    let mut parts = args.split_whitespace();
    let Some(first) = parts.next() else {
        return String::new();
    };
    let first_path = PathBuf::from(first);
    let first_name = first_path
        .file_name()
        .and_then(|name| name.to_str())
        .unwrap_or(first);
    if matches!(first_name, "codex" | ".codex-wrapped") {
        parts.collect::<Vec<_>>().join(" ")
    } else {
        args.to_string()
    }
}

fn collect_transcript_summaries() -> Vec<CodexTranscriptSummary> {
    let dir = sessions_dir();
    if let Ok(mut cached) = transcript_index_cache().lock() {
        if let Some(index) = cached.as_ref() {
            if index.sessions_dir == dir && index.refreshed_at.elapsed() < TRANSCRIPT_INDEX_TTL {
                return index.transcripts.clone();
            }
        }

        let transcripts = collect_transcript_summaries_uncached(&dir);
        *cached = Some(CachedTranscriptIndex {
            sessions_dir: dir,
            refreshed_at: Instant::now(),
            transcripts: transcripts.clone(),
        });
        return transcripts;
    }

    collect_transcript_summaries_uncached(&dir)
}

fn collect_transcript_summaries_uncached(dir: &PathBuf) -> Vec<CodexTranscriptSummary> {
    let mut paths = Vec::new();
    collect_rollout_jsonls(dir, &mut paths);

    let mut transcripts: Vec<CodexTranscriptSummary> = paths
        .into_iter()
        .filter_map(transcript_summary_from_codex_jsonl)
        .collect();
    transcripts.sort_by_key(|t| Reverse(t.mtime_ms));
    transcripts
}

fn sessions_from_live_processes(
    processes: Vec<LiveCodexProcess>,
    transcripts: &[CodexTranscriptSummary],
) -> Vec<CodexSession> {
    let mut sessions: Vec<CodexSession> = processes
        .into_iter()
        .map(|process| session_from_live_process(process, transcripts))
        .collect();
    sessions.sort_by_key(|s| Reverse(s.started_at));
    sessions
}

fn session_from_live_process(
    process: LiveCodexProcess,
    transcripts: &[CodexTranscriptSummary],
) -> CodexSession {
    let transcript = best_transcript_for_process(&process, transcripts);
    let session_id = transcript
        .map(|t| t.session_id.clone())
        .unwrap_or_else(|| format!("codex-{}", process.pid));

    let mut session = CodexSession::from_raw(crate::session::RawSession {
        pid: process.pid,
        session_id,
        cwd: process.cwd,
        started_at: process.started_at,
    });
    session.tty = process.tty;
    session.cpu_percent = process.cpu_percent;
    session.mem_mb = process.mem_mb;
    session.command_args = process.command_args;

    if let Some(transcript) = transcript {
        session.jsonl_path = Some(transcript.path.clone());
        session.last_message_ts = transcript.mtime_ms;
        session.model_profile_source = "codex-transcript".into();
    }

    session
}

fn best_transcript_for_process<'a>(
    process: &LiveCodexProcess,
    transcripts: &'a [CodexTranscriptSummary],
) -> Option<&'a CodexTranscriptSummary> {
    const START_TOLERANCE_MS: u64 = 10 * 60 * 1000;
    let min_mtime = process.started_at.saturating_sub(START_TOLERANCE_MS);

    transcripts
        .iter()
        .find(|t| t.cwd == process.cwd && t.mtime_ms >= min_mtime)
        .or_else(|| transcripts.iter().find(|t| t.cwd == process.cwd))
}

fn collect_rollout_jsonls(dir: &PathBuf, paths: &mut Vec<PathBuf>) {
    let Ok(entries) = fs::read_dir(dir) else {
        return;
    };

    for entry in entries.flatten() {
        let path = entry.path();
        if path.is_dir() {
            collect_rollout_jsonls(&path, paths);
            continue;
        }
        if path.extension().and_then(|e| e.to_str()) == Some("jsonl")
            && path
                .file_name()
                .and_then(|n| n.to_str())
                .is_some_and(|name| name.starts_with("rollout-"))
        {
            paths.push(path);
        }
    }
}

fn transcript_summary_from_codex_jsonl(path: PathBuf) -> Option<CodexTranscriptSummary> {
    let file = fs::File::open(&path).ok()?;
    let reader = std::io::BufReader::new(file);
    for line in reader.lines().map_while(Result::ok) {
        let Some(CodexEvent::SessionMeta(meta)) = parse_line(line.trim()) else {
            continue;
        };
        let mtime_ms = file_mtime_ms(&path).unwrap_or_default();
        return Some(CodexTranscriptSummary {
            session_id: meta.session_id,
            cwd: meta.cwd,
            path,
            mtime_ms,
        });
    }
    None
}

/// Resolve JSONL paths for sessions. Must be called AFTER command_args are populated
/// (i.e., after fetch_ps_data), so we can use resume UUIDs for correct mapping.
pub fn resolve_jsonl_paths(sessions: &mut [CodexSession]) {
    for session in sessions.iter_mut() {
        if !session.process_backed {
            continue;
        }
        let slug = cwd_to_slug(&session.cwd);
        let project_dir = projects_dir().join(&slug);

        // Priority 1: Try the session's own ID in the expected project dir
        let own_path = project_dir.join(format!("{}.jsonl", session.session_id));
        if own_path.exists() {
            session.jsonl_path = Some(own_path);
            continue;
        }

        // Priority 2: Try the resume UUID from command args
        if let Some(resume_id) = extract_resume_uuid(&session.command_args) {
            let resume_path = project_dir.join(format!("{resume_id}.jsonl"));
            if resume_path.exists() {
                session.jsonl_path = Some(resume_path);
                continue;
            }
        }

        // Priority 3: Fall back to most recently modified .jsonl in the project dir
        if let Some(latest) = find_latest_jsonl(&project_dir) {
            session.jsonl_path = Some(latest);
            continue;
        }

        // Priority 4: Search ALL project directories for a JSONL matching the session ID.
        // This handles cwd encoding mismatches between codexctl and Codex
        // (e.g., symlink resolution, path normalization differences).
        if let Some(found) = search_all_projects_for_session(&session.session_id) {
            crate::logger::log(
                "DEBUG",
                &format!(
                    "session {}: slug mismatch — found JSONL via project scan: {}",
                    session.session_id,
                    found.display()
                ),
            );
            session.jsonl_path = Some(found);
            continue;
        }

        let process = LiveCodexProcess {
            pid: session.pid,
            cwd: session.cwd.clone(),
            started_at: session.started_at,
            tty: session.tty.clone(),
            cpu_percent: session.cpu_percent,
            mem_mb: session.mem_mb,
            command_args: session.command_args.clone(),
        };
        let transcripts = collect_transcript_summaries();
        if let Some(transcript) = best_transcript_for_process(&process, &transcripts) {
            session.jsonl_path = Some(transcript.path.clone());
            session.last_message_ts = transcript.mtime_ms;
            session.model_profile_source = "codex-transcript".into();
            continue;
        }

        crate::logger::log(
            "DEBUG",
            &format!(
                "session {}: no JSONL found (slug={}, project_dir_exists={})",
                session.session_id,
                slug,
                project_dir.exists()
            ),
        );
    }
}

/// Search all directories under the Codex sessions root for a JSONL file matching the session ID.
/// This is a fallback when the cwd-based slug doesn't match the actual directory on disk.
fn search_all_projects_for_session(session_id: &str) -> Option<PathBuf> {
    let filename = format!("{session_id}.jsonl");
    let base = projects_dir();
    let entries = fs::read_dir(&base).ok()?;

    for entry in entries.flatten() {
        let path = entry.path();
        if !path.is_dir() {
            continue;
        }
        let candidate = path.join(&filename);
        if candidate.exists() {
            return Some(candidate);
        }
    }
    None
}

/// Extract the UUID from a resume argument in command args.
fn extract_resume_uuid(command_args: &str) -> Option<String> {
    let marker = if command_args.contains("--resume ") {
        "--resume "
    } else {
        "resume "
    };
    let start = command_args.find(marker)? + marker.len();
    let rest = &command_args[start..];
    // Take until whitespace — could be a UUID or a named session
    let token: String = rest.chars().take_while(|c| !c.is_whitespace()).collect();
    if token.is_empty() {
        return None;
    }
    // Strip surrounding quotes
    let token = token.trim_matches('"').trim_matches('\'');
    Some(token.to_string())
}

/// Find the most recently modified .jsonl file in a project directory.
fn find_latest_jsonl(dir: &PathBuf) -> Option<PathBuf> {
    let entries = fs::read_dir(dir).ok()?;
    let mut best: Option<(PathBuf, std::time::SystemTime)> = None;

    for entry in entries.flatten() {
        let path = entry.path();
        if path.extension().and_then(|e| e.to_str()) != Some("jsonl") {
            continue;
        }
        let modified = entry.metadata().ok()?.modified().ok()?;
        if best.as_ref().is_none_or(|(_, t)| modified > *t) {
            best = Some((path, modified));
        }
    }

    best.map(|(p, _)| p)
}

fn file_mtime_ms(path: &PathBuf) -> Option<u64> {
    let modified = fs::metadata(path).ok()?.modified().ok()?;
    Some(
        modified
            .duration_since(std::time::UNIX_EPOCH)
            .ok()?
            .as_millis() as u64,
    )
}

/// Feature #29: Scan for subagent task .jsonl files.
/// Legacy sub-agent task files live in:
///   /tmp/codex-{uid}/{project_slug}/{sessionId}/tasks/
pub fn scan_subagents(sessions: &mut [CodexSession]) {
    let uid = unsafe { libc::getuid() };
    let tmp_base = PathBuf::from(format!("/tmp/codex-{uid}"));

    if !tmp_base.exists() {
        for session in sessions.iter_mut() {
            session.active_subagent_count = 0;
            session.active_subagent_jsonl_paths.clear();
        }
        return;
    }

    for session in sessions.iter_mut() {
        if !session.process_backed {
            session.active_subagent_count = 0;
            session.active_subagent_jsonl_paths.clear();
            continue;
        }
        let slug = cwd_to_slug(&session.cwd);
        let tasks_dir = tmp_base.join(&slug).join(&session.session_id).join("tasks");

        if !tasks_dir.exists() {
            session.active_subagent_count = 0;
            session.active_subagent_jsonl_paths.clear();
            continue;
        }

        let mut jsonls = Vec::new();
        collect_subagent_jsonls(&tasks_dir, &mut jsonls);
        jsonls.sort();
        session.active_subagent_count = jsonls.len();
        session.active_subagent_jsonl_paths = jsonls;
    }
}

fn collect_subagent_jsonls(dir: &PathBuf, jsonls: &mut Vec<PathBuf>) {
    let Ok(entries) = fs::read_dir(dir) else {
        return;
    };

    for entry in entries.flatten() {
        let path = entry.path();
        if path.is_dir() {
            collect_subagent_jsonls(&path, jsonls);
            continue;
        }
        if path.extension().and_then(|ext| ext.to_str()) == Some("jsonl") {
            jsonls.push(path);
        }
    }
}

/// Resolve git worktree identity for each session (for conflict detection).
/// Sessions in different worktrees of the same repo get different IDs.
/// Runs `git rev-parse --show-toplevel` once per unique cwd.
pub fn resolve_worktree_ids(sessions: &mut [CodexSession]) {
    // Cache results to avoid running git multiple times for the same cwd
    let mut cache: std::collections::HashMap<String, String> = std::collections::HashMap::new();

    for session in sessions.iter_mut() {
        if session.worktree_id.is_some() {
            continue;
        }
        let id = if let Some(cached) = cache.get(&session.cwd) {
            cached.clone()
        } else {
            let resolved = std::process::Command::new("git")
                .args(["rev-parse", "--show-toplevel"])
                .current_dir(&session.cwd)
                .output()
                .ok()
                .and_then(|o| {
                    if o.status.success() {
                        String::from_utf8(o.stdout)
                            .ok()
                            .map(|s| s.trim().to_string())
                    } else {
                        None
                    }
                })
                // Fall back to cwd if not a git repo
                .unwrap_or_else(|| session.cwd.clone());
            cache.insert(session.cwd.clone(), resolved.clone());
            resolved
        };
        session.worktree_id = Some(id);
    }
}

fn cwd_to_slug(cwd: &str) -> String {
    let trimmed = cwd.trim_end_matches('/');
    if trimmed.is_empty() {
        return "-".to_string();
    }
    trimmed.replace('/', "-")
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn slug_basic_path() {
        assert_eq!(cwd_to_slug("/Users/foo/bar"), "-Users-foo-bar");
    }

    #[test]
    fn slug_trailing_slash() {
        // Must strip trailing slash — otherwise slug ends with "-" and won't match disk
        assert_eq!(
            cwd_to_slug("/Users/foo/bar/"),
            "-Users-foo-bar",
            "trailing slash must be stripped before slugifying"
        );
    }

    #[test]
    fn slug_multiple_trailing_slashes() {
        assert_eq!(cwd_to_slug("/Users/foo/bar///"), "-Users-foo-bar");
    }

    #[test]
    fn slug_with_hyphens_in_name() {
        assert_eq!(
            cwd_to_slug("/Users/dev/data-platform-answers"),
            "-Users-dev-data-platform-answers"
        );
    }

    #[test]
    fn slug_root() {
        assert_eq!(cwd_to_slug("/"), "-");
    }

    #[test]
    fn slug_single_component() {
        assert_eq!(cwd_to_slug("/tmp"), "-tmp");
    }

    #[test]
    fn transcript_history_without_live_processes_yields_no_sessions() {
        let transcript = CodexTranscriptSummary {
            session_id: "sess-history".into(),
            cwd: "/repo".into(),
            path: PathBuf::from("/tmp/rollout-history.jsonl"),
            mtime_ms: 10_000,
        };

        let sessions = sessions_from_live_processes(Vec::new(), &[transcript]);

        assert!(sessions.is_empty());
    }

    #[test]
    fn live_process_attaches_matching_recent_transcript() {
        let transcript = CodexTranscriptSummary {
            session_id: "sess-live".into(),
            cwd: "/repo".into(),
            path: PathBuf::from("/tmp/rollout-live.jsonl"),
            mtime_ms: 120_000,
        };
        let process = LiveCodexProcess {
            pid: 42,
            cwd: "/repo".into(),
            started_at: 100_000,
            tty: "pts/1".into(),
            cpu_percent: 3.5,
            mem_mb: 64.0,
            command_args: String::new(),
        };

        let sessions = sessions_from_live_processes(vec![process], &[transcript]);

        assert_eq!(sessions.len(), 1);
        assert!(sessions[0].process_backed);
        assert_eq!(sessions[0].pid, 42);
        assert_eq!(sessions[0].session_id, "sess-live");
        assert_eq!(sessions[0].cwd, "/repo");
        assert_eq!(
            sessions[0].jsonl_path.as_deref(),
            Some(std::path::Path::new("/tmp/rollout-live.jsonl"))
        );
    }

    #[test]
    fn live_process_without_matching_transcript_still_appears_once() {
        let transcript = CodexTranscriptSummary {
            session_id: "sess-other".into(),
            cwd: "/other".into(),
            path: PathBuf::from("/tmp/rollout-other.jsonl"),
            mtime_ms: 120_000,
        };
        let process = LiveCodexProcess {
            pid: 99,
            cwd: "/repo".into(),
            started_at: 100_000,
            tty: "pts/2".into(),
            cpu_percent: 0.0,
            mem_mb: 32.0,
            command_args: String::new(),
        };

        let sessions = sessions_from_live_processes(vec![process], &[transcript]);

        assert_eq!(sessions.len(), 1);
        assert_eq!(sessions[0].session_id, "codex-99");
        assert_eq!(sessions[0].jsonl_path, None);
    }

    #[test]
    fn transcript_summary_scan_reuses_fresh_index() {
        let dir = tempfile::tempdir().unwrap();
        let codex_home = dir.path().join(".codex");
        let first = codex_home
            .join("sessions")
            .join("2026")
            .join("07")
            .join("07")
            .join("rollout-first.jsonl");
        write_transcript(&first, "sess-first", "/repo");

        unsafe {
            std::env::set_var("CODEXCTL_CODEX_HOME", &codex_home);
        }
        let summaries = collect_transcript_summaries();
        assert_eq!(summaries.len(), 1);
        assert_eq!(summaries[0].session_id, "sess-first");

        let second = codex_home
            .join("sessions")
            .join("2026")
            .join("07")
            .join("07")
            .join("rollout-second.jsonl");
        write_transcript(&second, "sess-second", "/repo");

        let summaries = collect_transcript_summaries();
        unsafe {
            std::env::remove_var("CODEXCTL_CODEX_HOME");
        }

        assert_eq!(summaries.len(), 1);
        assert_eq!(summaries[0].session_id, "sess-first");
    }

    fn write_transcript(path: &std::path::Path, session_id: &str, cwd: &str) {
        std::fs::create_dir_all(path.parent().unwrap()).unwrap();
        std::fs::write(
            path,
            format!(
                r#"{{"timestamp":"2026-07-07T00:00:00Z","type":"session_meta","payload":{{"id":"{session_id}","timestamp":"2026-07-07T00:00:00Z","cwd":"{cwd}","model_provider":"openai"}}}}"#
            ),
        )
        .unwrap();
    }
}
