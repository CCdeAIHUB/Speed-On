use std::process::Command;

use speed_on_core::{
    AppError, AppResult, OpenResourceOutcome, OpenResourceRequest, ResourceKind, ResourceOpener,
};

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct CommandPlan {
    pub program: String,
    pub args: Vec<String>,
}

impl CommandPlan {
    pub fn new(program: impl Into<String>, args: Vec<String>) -> Self {
        Self {
            program: program.into(),
            args,
        }
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct OpenTargetValidator;

impl OpenTargetValidator {
    pub fn validate(request: &OpenResourceRequest) -> AppResult<()> {
        let target = request.resource.target.trim();
        if target.is_empty() {
            return Err(AppError::invalid_argument(
                "open resource target must not be empty",
                "platform::OpenTargetValidator",
            ));
        }

        if target.chars().any(char::is_control) {
            return Err(AppError::invalid_argument(
                "open resource target must not contain control characters",
                "platform::OpenTargetValidator",
            ));
        }

        if request.resource.kind == ResourceKind::BrowserUrl {
            validate_browser_url_scheme(target)?;
        }

        Ok(())
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct PlatformCommandPlanner;

impl PlatformCommandPlanner {
    pub fn plan(request: &OpenResourceRequest) -> AppResult<CommandPlan> {
        OpenTargetValidator::validate(request)?;
        Ok(plan_for_current_platform(&request.resource.target))
    }
}

pub trait CommandRunner {
    fn run(&mut self, plan: &CommandPlan) -> AppResult<()>;
}

#[derive(Debug, Default, Clone, Copy, PartialEq, Eq)]
pub struct ProcessCommandRunner;

impl CommandRunner for ProcessCommandRunner {
    fn run(&mut self, plan: &CommandPlan) -> AppResult<()> {
        // The plan is executed with `Command` arguments instead of shell string
        // concatenation. This keeps target paths and URLs as data arguments and
        // avoids the shell-injection class of bugs for this first platform adapter.
        let status = Command::new(&plan.program)
            .args(&plan.args)
            .status()
            .map_err(|error| {
                AppError::platform_unsupported(
                    "failed to start platform open command",
                    "platform::ProcessCommandRunner",
                )
                .with_cause(error.to_string())
            })?;

        if status.success() {
            Ok(())
        } else {
            Err(AppError::platform_unsupported(
                format!("platform open command exited with status: {status}"),
                "platform::ProcessCommandRunner",
            ))
        }
    }
}

pub struct CommandResourceOpener<R>
where
    R: CommandRunner,
{
    runner: R,
}

impl<R> CommandResourceOpener<R>
where
    R: CommandRunner,
{
    pub fn new(runner: R) -> Self {
        Self { runner }
    }
}

impl Default for CommandResourceOpener<ProcessCommandRunner> {
    fn default() -> Self {
        Self::new(ProcessCommandRunner)
    }
}

impl<R> ResourceOpener for CommandResourceOpener<R>
where
    R: CommandRunner,
{
    fn open_resource(&mut self, request: &OpenResourceRequest) -> AppResult<OpenResourceOutcome> {
        let plan = PlatformCommandPlanner::plan(request)?;
        self.runner.run(&plan)?;

        Ok(OpenResourceOutcome {
            resource_id: request.resource.id.clone(),
            kind: request.resource.kind,
            target: request.resource.target.clone(),
            opened_at_millis: request.requested_at_millis,
        })
    }
}

fn validate_browser_url_scheme(target: &str) -> AppResult<()> {
    let normalized = target.to_ascii_lowercase();
    let allowed = normalized.starts_with("http://")
        || normalized.starts_with("https://")
        || normalized.starts_with("file://");

    if allowed {
        Ok(())
    } else {
        Err(AppError::invalid_argument(
            "browser URL scheme is not allowed for open_resource",
            "platform::OpenTargetValidator",
        ))
    }
}

fn plan_for_current_platform(target: &str) -> CommandPlan {
    plan_for_platform(std::env::consts::OS, target)
}

pub fn plan_for_platform(os: &str, target: &str) -> CommandPlan {
    match os {
        "macos" => CommandPlan::new("open", vec![target.to_owned()]),
        "windows" => CommandPlan::new("explorer", vec![target.to_owned()]),
        "linux" => CommandPlan::new("xdg-open", vec![target.to_owned()]),
        _ => CommandPlan::new("xdg-open", vec![target.to_owned()]),
    }
}
