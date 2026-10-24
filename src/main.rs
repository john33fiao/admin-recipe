use anyhow::{Context, Result};
use std::path::{Path, PathBuf};
use sunchang_hansang::{
    lifecycle::Policy,
    server::{AppState, router},
    store::Store,
};

fn data_directory(executable: &Path) -> Result<PathBuf> {
    Ok(executable
        .parent()
        .context("실행파일의 폴더를 확인할 수 없습니다.")?
        .join("db"))
}

#[tokio::main]
async fn main() {
    if let Err(error) = run().await {
        eprintln!("순창한상을 실행하지 못했습니다: {error:#}");
        std::process::exit(1);
    }
}
async fn run() -> Result<()> {
    let args: Vec<String> = std::env::args().skip(1).collect();
    if args.iter().any(|arg| arg == "--help" || arg == "-h") {
        println!(
            "순창한상\n사용법: sunchang-hansang [--no-browser]\n데이터: 실행파일 옆 db/\n--no-browser: 자동 열기 없이 주소 표시 (검증·수동 실행용)"
        );
        return Ok(());
    }
    anyhow::ensure!(
        args.iter().all(|arg| arg == "--no-browser"),
        "알 수 없는 실행 인수입니다. --help를 확인해 주세요."
    );
    let root = data_directory(&std::env::current_exe().context("실행파일 경로 확인 실패")?)?;
    let store = Store::open(&root)
        .context("db/를 열지 못했습니다. 폴더 쓰기 권한과 이미 실행 중인 앱을 확인해 주세요.")?;
    let listener = tokio::net::TcpListener::bind("127.0.0.1:0")
        .await
        .context("로컬 서버 시작 실패")?;
    let address = listener.local_addr()?;
    let token = format!(
        "{}{}",
        uuid::Uuid::new_v4().simple(),
        uuid::Uuid::new_v4().simple()
    );
    let url = format!("http://{address}/#token={token}");
    let state = AppState::new(store, address.to_string(), token, Policy::default());
    let watchdog_state = state.clone();
    let watchdog = tokio::spawn(async move {
        watchdog_state.watchdog().await;
    });
    let stop_state = state.clone();
    let signal = tokio::spawn(async move {
        if tokio::signal::ctrl_c().await.is_ok() {
            stop_state.shutdown();
        }
    });
    let app = router(state.clone());
    let shutdown_state = state.clone();
    let server = tokio::spawn(async move {
        axum::serve(listener, app)
            .with_graceful_shutdown(async move {
                shutdown_state.cancelled().await;
            })
            .await
    });
    println!("순창한상 주소: {url}");
    println!(
        "모든 화면의 연결이 끊기면 유예 후 종료합니다. 종료 시에는 저장 중인 작업을 기다립니다."
    );
    if !args.iter().any(|arg| arg == "--no-browser") {
        let open_url = url.clone();
        match tokio::task::spawn_blocking(move || webbrowser::open(&open_url)).await {
            Ok(Ok(_)) => {}
            result => eprintln!(
                "브라우저를 자동으로 열지 못했습니다 ({result:?}). 위 주소를 직접 열어 주세요. 최초 접속 대기는 120초입니다."
            ),
        }
    }
    server.await.context("서버 작업 실패")??;
    state.shutdown();
    state.wait_for_sockets().await;
    watchdog.await?;
    signal.abort();
    println!("순창한상을 안전하게 종료했습니다.");
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn data_path_uses_executable_parent() {
        let base = std::env::temp_dir().join("한글 폴더");
        assert_eq!(data_directory(&base.join("app")).unwrap(), base.join("db"));
    }
}
