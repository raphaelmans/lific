//! The supported CLI for this personal fork. Upstream command types remain
//! internal adapters; they are never registered as executable commands.
use crate::cli::{
    self, Command, CommentAction, ExportAction, FolderAction, IssueAction, LabelAction,
    ModuleAction, PageAction, ProjectAction, ServiceAction,
};
use clap::{Parser, Subcommand};
use std::path::PathBuf;

#[derive(Parser)]
#[command(name = "lific", version, about = "Personal, local-only issue tracker")]
pub struct PersonalCli {
    /// Local configuration file
    #[arg(long, global = true)]
    config: Option<PathBuf>,
    /// Local SQLite database (overrides config)
    #[arg(long, global = true)]
    db: Option<PathBuf>,
    /// Read SQLite directly, or use the loopback HTTP API
    #[arg(long, global = true, value_enum, default_value_t = cli::BackendKind::Sql)]
    backend: cli::BackendKind,
    /// Local server URL (only loopback addresses are accepted)
    #[arg(long, global = true, value_parser = local_server_url)]
    url: Option<String>,
    /// Optional local tool token for attribution (also LIFIC_TOKEN)
    #[arg(long, global = true, env = "LIFIC_TOKEN", hide_env_values = true)]
    api_key: Option<String>,
    /// Output JSON
    #[arg(long, global = true)]
    json: bool,
    #[command(subcommand)]
    command: PersonalCommand,
}

#[derive(Subcommand)]
enum PersonalCommand {
    /// Open the personal web app on this computer
    Start {
        #[arg(short, long)]
        port: Option<u16>,
    },
    /// Serve MCP over stdio, optionally through the local HTTP MCP endpoint
    Mcp {
        /// Proxy stdio to the loopback server's /mcp endpoint
        #[arg(long)]
        http: bool,
    },
    /// Check local configuration, database, backups, and app health
    Doctor {
        #[arg(long)]
        repair: bool,
    },
    /// Create your login-free local workspace
    Init {
        #[arg(long)]
        no_service: bool,
        #[arg(long)]
        here: bool,
        /// Display name for local authorship (no account or password needed)
        #[arg(long, default_value = "local-operator")]
        name: String,
    },
    /// Manage the local background process
    Service {
        #[command(subcommand)]
        action: ServiceAction,
    },
    /// Back up the local database and attachments
    Dump {
        #[arg(long)]
        out: Option<PathBuf>,
    },
    /// Restore a local backup (stop the app first)
    Restore {
        archive: PathBuf,
        #[arg(long)]
        force: bool,
        #[arg(long)]
        allow_large: bool,
    },
    /// Configure an installed MCP client for local stdio or loopback HTTP
    Connect {
        /// Use the loopback HTTP MCP endpoint instead of local stdio
        #[arg(long)]
        http: bool,
        /// Use local stdio (the default)
        #[arg(long, conflicts_with = "http")]
        stdio: bool,
        #[arg(long = "client")]
        clients: Vec<String>,
        #[arg(long, default_value = "global")]
        scope: String,
        #[arg(long)]
        yes: bool,
        #[arg(long)]
        dry_run: bool,
        #[arg(long)]
        skip_agents: bool,
    },
    /// Import provider issue JSON from a local file; never contacts a provider
    ImportFile {
        file: PathBuf,
        #[arg(long, value_enum)]
        source: crate::import::file::Source,
        #[arg(long)]
        project: String,
        /// GitHub owner/repo or Jira site label, used only for deduplication
        #[arg(long)]
        namespace: Option<String>,
        #[arg(long)]
        dry_run: bool,
    },
    /// Move a project using a local archive file
    ProjectArchive {
        #[command(subcommand)]
        action: PersonalArchiveAction,
    },
    /// Write or update the Lific block in a project's AGENTS.md so agents in
    /// that repo know it uses Lific for issue tracking (LIF-251).
    ///
    /// Idempotent: re-running replaces the marker-delimited block in place and
    /// preserves everything around it. Creates AGENTS.md if missing.
    AgentsMd {
        /// Path to the AGENTS.md file (default: ./AGENTS.md).
        #[arg(long)]
        path: Option<PathBuf>,

        /// Project identifier to bake into the CLI examples (e.g. LIF). If
        /// omitted, a generic placeholder is used with a discovery note.
        #[arg(long)]
        project: Option<String>,
    },

    /// Bind the repository containing the current directory to a project, so
    /// tools and agents can tell which project a checkout belongs to (LIF-450).
    ///
    /// With no PROJECT this reports what the repository currently resolves to,
    /// which doubles as the "where am I" diagnostic. Identity comes from the
    /// repository's `origin` remote and its root commit, never from the path,
    /// so a second clone of the same repository resolves to the same project
    /// without being bound again.
    ///
    Bind {
        /// Project identifier to bind this repository to (e.g. LIF). Omit to
        /// report what the repository already resolves to.
        project: Option<String>,

        /// Create the project first if it does not exist yet.
        #[arg(long)]
        create: bool,
    },

    /// Close the issues that commit messages say they close (LIF-5).
    ///
    /// A message containing `closes LIF-42` (or `fixes`, or `resolves`, in any
    /// case) marks LIF-42 done. A bare mention does not: "see LIF-42" leaves
    /// the issue alone.
    ///
    /// With no `--range`, the whole of stdin is read as one message, which is
    /// what a `commit-msg` hook wants:
    ///
    ///   #!/bin/sh
    ///   lific git-hook < "$1"
    ///
    /// With `--range`, the messages come from `git log` over that range, which
    /// is what a `post-receive` hook or a CI step wants:
    ///
    ///   lific git-hook --range origin/master..HEAD
    ///
    GitHook {
        /// Commit range to read messages from (e.g. `origin/master..HEAD`).
        /// Omit to read one message from stdin.
        #[arg(long)]
        range: Option<String>,

        /// Report what would close and write nothing.
        #[arg(long)]
        dry_run: bool,
    },

    /// Generate shell completions (e.g. `lific completion fish | source`)
    Completion {
        /// Shell to generate completions for (bash, zsh, fish, powershell, elvish)
        shell: clap_complete::Shell,
    },

    /// Manage issues
    Issue {
        #[command(subcommand)]
        action: IssueAction,
    },

    /// Manage projects
    Project {
        #[command(subcommand)]
        action: ProjectAction,
    },

    /// Manage pages (documentation)
    Page {
        #[command(subcommand)]
        action: PageAction,
    },

    /// Export issues and pages as markdown
    Export {
        #[command(subcommand)]
        action: ExportAction,
    },

    /// Search issues, pages, and comments
    Search {
        /// Search query text
        query: String,

        /// Filter to a specific project (identifier, e.g. LIF)
        #[arg(short, long)]
        project: Option<String>,

        /// Max results (default 20)
        #[arg(short, long)]
        limit: Option<i64>,
    },

    /// Manage comments on issues
    Comment {
        #[command(subcommand)]
        action: CommentAction,
    },

    /// Manage modules
    Module {
        #[command(subcommand)]
        action: ModuleAction,
    },

    /// Manage labels
    Label {
        #[command(subcommand)]
        action: LabelAction,
    },

    /// Manage folders
    Folder {
        #[command(subcommand)]
        action: FolderAction,
    },
}

#[derive(Subcommand)]
enum PersonalArchiveAction {
    /// Export a project, including attachments and history
    Export {
        project: String,
        #[arg(long)]
        out: PathBuf,
    },
    /// Import a project into your local workspace
    Import { archive: PathBuf },
}

impl From<PersonalCli> for cli::Cli {
    fn from(value: PersonalCli) -> Self {
        let command = match value.command {
            PersonalCommand::Start { port } => Command::Start {
                port,
                host: None,
                init_if_missing: false,
            },
            PersonalCommand::Mcp { http } => Command::Mcp {
                remote: http,
                url: None,
                instances: None,
            },
            PersonalCommand::Doctor { repair } => Command::Doctor { key: None, repair },
            PersonalCommand::Init {
                no_service,
                here,
                name,
            } => Command::Init {
                no_service,
                here,
                name: Some(name),
                auth_mode: Some("login-free".into()),
                password: None,
            },
            PersonalCommand::Service { action } => Command::Service { action },
            PersonalCommand::Dump { out } => Command::Dump { out },
            PersonalCommand::Restore {
                archive,
                force,
                allow_large,
            } => Command::Restore {
                archive,
                force,
                allow_large,
            },
            PersonalCommand::Connect {
                http,
                stdio: _,
                clients,
                scope,
                yes,
                dry_run,
                skip_agents,
            } => Command::Connect {
                clients,
                scope,
                yes,
                dry_run,
                skip_agents,
                stdio: !http,
                oauth: false,
                url: value.url.clone(),
                key: value.api_key.clone(),
                user: None,
            },
            PersonalCommand::ImportFile {
                file,
                source,
                project,
                namespace,
                dry_run,
            } => Command::Import {
                action: cli::ImportAction::File {
                    file,
                    source,
                    project,
                    namespace,
                    dry_run,
                },
            },
            PersonalCommand::ProjectArchive { action } => Command::ProjectArchive {
                action: match action {
                    PersonalArchiveAction::Export { project, out } => {
                        cli::ProjectArchiveAction::Export { project, out }
                    }
                    PersonalArchiveAction::Import { archive } => {
                        cli::ProjectArchiveAction::Import {
                            archive,
                            user: String::new(),
                        }
                    }
                },
            },
            PersonalCommand::AgentsMd { path, project } => Command::AgentsMd { path, project },
            PersonalCommand::Bind { project, create } => Command::Bind { project, create },
            PersonalCommand::GitHook { range, dry_run } => Command::GitHook { range, dry_run },
            PersonalCommand::Completion { shell } => Command::Completion { shell },
            PersonalCommand::Issue { action } => Command::Issue { action },
            PersonalCommand::Project { action } => Command::Project { action },
            PersonalCommand::Page { action } => Command::Page { action },
            PersonalCommand::Export { action } => Command::Export { action },
            PersonalCommand::Search {
                query,
                project,
                limit,
            } => Command::Search {
                query,
                project,
                limit,
            },
            PersonalCommand::Comment { action } => Command::Comment { action },
            PersonalCommand::Module { action } => Command::Module { action },
            PersonalCommand::Label { action } => Command::Label { action },
            PersonalCommand::Folder { action } => Command::Folder { action },
        };
        Self {
            config: value.config,
            db: value.db,
            json: value.json,
            backend: value.backend,
            url: value.url,
            api_key: value.api_key,
            command,
        }
    }
}

fn local_server_url(value: &str) -> Result<String, String> {
    crate::network::local_url(value)?;
    Ok(value.to_owned())
}

#[cfg(test)]
mod tests {
    use super::*;
    use clap::CommandFactory;

    #[test]
    fn help_and_completion_use_only_the_personal_command_registry() {
        let help = PersonalCli::command().render_long_help().to_string();
        for removed in [
            "login", "logout", "user", "member", "key", "instance", "import",
        ] {
            assert!(
                !PersonalCli::command()
                    .get_subcommands()
                    .any(|cmd| cmd.get_name() == removed)
            );
        }
        for flag in ["--backend", "--url", "--api-key"] {
            assert!(help.contains(flag));
        }
    }

    #[test]
    fn local_tools_cannot_choose_remote_or_multiple_instances() {
        let cli: cli::Cli = PersonalCli::try_parse_from([
            "lific",
            "connect",
            "--client",
            "cursor",
            "--dry-run",
            "--yes",
        ])
        .unwrap()
        .into();
        assert!(matches!(
            cli.command,
            Command::Connect {
                stdio: true,
                oauth: false,
                url: None,
                key: None,
                user: None,
                ..
            }
        ));
        let cli: cli::Cli = PersonalCli::try_parse_from(["lific", "mcp"])
            .unwrap()
            .into();
        assert!(matches!(
            cli.command,
            Command::Mcp {
                remote: false,
                url: None,
                instances: None
            }
        ));
        let cli: cli::Cli = PersonalCli::try_parse_from(["lific", "init", "--no-service"])
            .unwrap()
            .into();
        assert!(
            matches!(cli.command, Command::Init { password: None, auth_mode: Some(mode), .. } if mode == "login-free")
        );
    }
}
