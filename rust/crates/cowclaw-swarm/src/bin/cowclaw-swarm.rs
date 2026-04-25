use cowclaw_swarm::mcp_stdio::server::McpServer;
use cowclaw_swarm::planner::artifacts::PlanningTree;
use std::fmt::Write as _;
use std::path::{Path, PathBuf};

#[tokio::main]
async fn main() {
    let args: Vec<String> = std::env::args().collect();

    if args.iter().any(|a| a == "--mcp-stdio") {
        let root = PathBuf::from(
            args.iter()
                .position(|a| a == "--root")
                .and_then(|i| args.get(i + 1))
                .map_or(".cowclaw/planning", String::as_str),
        );
        let db_path = PathBuf::from(
            args.iter()
                .position(|a| a == "--db")
                .and_then(|i| args.get(i + 1))
                .map_or(".cowclaw/swarm.db", String::as_str),
        );
        let server = McpServer::new(root, db_path);
        let mut stdin = tokio::io::stdin();
        let mut stdout = tokio::io::stdout();
        if let Err(e) = server.serve(&mut stdin, &mut stdout).await {
            eprintln!("[cowclaw-swarm] error: {e}");
            std::process::exit(1);
        }
    } else if args.iter().any(|a| a == "--dump-planning-graph") {
        let root = PathBuf::from(
            args.iter()
                .position(|a| a == "--root")
                .and_then(|i| args.get(i + 1))
                .map_or(".cowclaw/planning", String::as_str),
        );
        match dump_planning_graph(&root) {
            Ok(mermaid) => println!("{mermaid}"),
            Err(e) => {
                eprintln!("[cowclaw-swarm] error: {e}");
                std::process::exit(1);
            }
        }
    } else if args.iter().any(|a| a == "--classify-only") {
        let objective = args
            .iter()
            .position(|a| a == "--objective")
            .and_then(|i| args.get(i + 1))
            .map_or("", String::as_str);

        let files_str = args
            .iter()
            .position(|a| a == "--files")
            .and_then(|i| args.get(i + 1))
            .map_or("", String::as_str);

        let files: Vec<&str> = if files_str.is_empty() {
            vec![]
        } else {
            files_str.split(',').collect()
        };

        let result = cowclaw_swarm::planner::classify::classify(objective, &files);
        match serde_json::to_string_pretty(&result) {
            Ok(json) => println!("{json}"),
            Err(e) => {
                eprintln!("[cowclaw-swarm] JSON serialization error: {e}");
                std::process::exit(1);
            }
        }
    } else {
        eprintln!("Usage: cowclaw-swarm --mcp-stdio [--root <dir>] [--db <path>]");
        eprintln!("       cowclaw-swarm --dump-planning-graph [--root <dir>]");
        eprintln!("       cowclaw-swarm --classify-only --objective <text> [--files <comma-separated-paths>]");
        std::process::exit(1);
    }
}

fn dump_planning_graph(root: &Path) -> cowclaw_swarm::Result<String> {
    let phases = PlanningTree::load(root)?;
    let mut out = String::from("graph TD\n");
    for phase in &phases {
        let _ = writeln!(out, "  {}[\"Phase: {}\"]", phase.id, phase.id);
        for wave in &phase.waves {
            let wave_node = wave.id.replace('/', "_");
            let _ = writeln!(out, "  {} --> {}[\"Wave: {}\"]", phase.id, wave_node, wave.id);
            for plan in &wave.plans {
                let plan_node = plan.id.replace('/', "_");
                let _ = writeln!(out, "  {} --> {}[\"Plan: {}\"]", wave_node, plan_node, plan.id);
            }
        }
    }
    if phases.is_empty() {
        out.push_str("  empty[\"No phases found\"]\n");
    }
    Ok(out)
}
