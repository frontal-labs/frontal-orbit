//! A single [`Thread`] conversation with the Orbit agent.

use std::sync::Arc;

use serde_json::Value;
use tokio::sync::Mutex;

use crate::config::{config_to_args, merge_config};
use crate::orbit::Orbit;
use crate::protocol::{ThreadInput, ThreadOptions, ThreadRunOptions, TurnResult};
use crate::spawn::{run_turn_buffered, run_turn_streamed, StreamedTurn};

/// A single conversation with the Orbit agent.
#[derive(Clone)]
pub struct Thread {
    orbit: Arc<Orbit>,
    options: ThreadOptions,
    session_id: Arc<Mutex<Option<String>>>,
}

impl Thread {
    pub(crate) fn new(
        orbit: Arc<Orbit>,
        options: ThreadOptions,
        session_id: Option<String>,
    ) -> Self {
        Self {
            orbit,
            options,
            session_id: Arc::new(Mutex::new(session_id)),
        }
    }

    /// The persisted session id, once known.
    pub async fn id(&self) -> Option<String> {
        self.session_id.lock().await.clone()
    }

    /// Run a turn and buffer the result. Call repeatedly on the same instance
    /// to continue the conversation (the CLI is resumed via its session id).
    pub async fn run(
        &self,
        input: &ThreadInput,
        run_options: &ThreadRunOptions,
    ) -> Result<TurnResult, crate::spawn::OrbitError> {
        let sid = self.session_id.lock().await.clone();
        let args = self.build_args(input, run_options, sid.as_deref());
        let env = self.orbit.build_env();
        let turn = run_turn_buffered(
            &self.orbit.command,
            &args,
            env,
            self.options.working_directory.as_ref(),
        )
        .await?;
        if turn.session_id.is_some() {
            *self.session_id.lock().await = turn.session_id.clone();
        }
        Ok(turn.result)
    }

    /// Run a turn and stream structured events as they arrive. Await
    /// [`StreamedTurn::finish`] after draining `events` to surface CLI errors,
    /// and call [`Thread::id`] to read the captured session id.
    pub async fn run_streamed(
        &self,
        input: &ThreadInput,
        run_options: &ThreadRunOptions,
    ) -> Result<StreamedTurn, crate::spawn::OrbitError> {
        let sid = self.session_id.lock().await.clone();
        let args = self.build_args(input, run_options, sid.as_deref());
        let env = self.orbit.build_env();
        run_turn_streamed(
            &self.orbit.command,
            &args,
            env,
            self.options.working_directory.as_ref(),
            Arc::clone(&self.session_id),
        )
        .await
    }

    fn build_args(
        &self,
        input: &ThreadInput,
        run_options: &ThreadRunOptions,
        session_id: Option<&str>,
    ) -> Vec<String> {
        let (prompt, images) = input.to_prompt_and_images();

        let mut args = vec!["prompt".to_string(), "-p".to_string(), prompt];
        for image in &images {
            args.push("--image".to_string());
            args.push(image.clone());
        }
        if let Some(provider) = &self.options.provider {
            args.push("--provider".to_string());
            args.push(provider.clone());
        }
        if let Some(model) = &self.options.model {
            args.push("--model".to_string());
            args.push(model.clone());
        }
        if let Some(permission_mode) = &self.options.permission_mode {
            args.push("--permission-mode".to_string());
            args.push(permission_mode.clone());
        }
        if let Some(id) = session_id {
            args.push("--resume".to_string());
            args.push(id.to_string());
        }
        args.push("--output-format".to_string());
        args.push("json".to_string());
        args.push("--stream".to_string());

        let config = self.merge_config(run_options);
        args.extend(config_to_args(&config));
        args
    }

    fn merge_config(&self, run_options: &ThreadRunOptions) -> Value {
        let mut config = self
            .orbit
            .options
            .config
            .clone()
            .unwrap_or(Value::Object(Default::default()));

        if let Some(base_url) = &self.orbit.options.base_url {
            config = merge_config(
                &config,
                &serde_json::json!({ "frontal_base_url": base_url }),
            );
        }
        if self.options.skip_git_repo_check {
            config = merge_config(&config, &serde_json::json!({ "skip_git_repo_check": true }));
        }
        if let Some(thread_config) = &self.options.config {
            config = merge_config(&config, thread_config);
        }
        if let Some(run_config) = &run_options.config {
            config = merge_config(&config, run_config);
        }
        if let Some(schema) = &run_options.output_schema {
            config = merge_config(
                &config,
                &serde_json::json!({ "output_schema": serde_json::to_string(schema).unwrap_or_default() }),
            );
        }
        config
    }
}
