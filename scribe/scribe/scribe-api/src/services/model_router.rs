use uuid::Uuid;
use crate::models::UsageError;
use crate::services::usage::UsageService;

#[derive(Clone)]
pub struct ModelRouter {
    usage_service: UsageService,
}

impl ModelRouter {
    pub fn new(usage_service: UsageService) -> Self {
        Self { usage_service }
    }

    // ============================================
    // MODEL SELECTION BY PLAN
    // ============================================

    /// Route model based on user's plan and current usage
    pub async fn route_model(
        &self,
        user_id: Uuid,
        requested_model: Option<String>,
        task_type: TaskType,
    ) -> Result<String, UsageError> {
        // Get user stats
        let usage_stats = self.usage_service.get_user_usage(user_id).await?;

        // Model cost protection: downgrade to cheapest when approaching limit
        if usage_stats.percentage_used >= 90.0 {
            return Ok(self.get_cheapest_model(&usage_stats.plan));
        }
        // Pro plan: fallback to mini at 80% to protect margins
        if usage_stats.percentage_used >= 80.0 && usage_stats.plan == "pro" {
            return Ok(self.get_cheapest_model(&usage_stats.plan));
        }

        // If user requested a specific model, validate it's allowed for their plan
        if let Some(model) = requested_model {
            if self.is_model_allowed_for_plan(&model, &usage_stats.plan) {
                return Ok(model);
            }
            // If not allowed, fall back to plan default
        }

        // Route based on task type and plan
        Ok(self.get_model_for_task(&usage_stats.plan, task_type))
    }

    /// Get default model for a plan and task type
    fn get_model_for_task(&self, plan: &str, task_type: TaskType) -> String {
        match (plan, task_type) {
            // Free plan: Always cheapest model
            ("free", _) => "gpt-4o-mini".to_string(),

            // Starter plan: Always gpt-4o-mini (cost protection)
            ("starter", _) => "gpt-4o-mini".to_string(),

            // Pro plan: Mid-tier for chat, premium for code
            ("pro", TaskType::Chat) => "gpt-4o".to_string(),
            ("pro", TaskType::Code) => "claude-3-5-sonnet".to_string(),
            ("pro", TaskType::Analysis) => "gpt-4o".to_string(),

            // Power plan: Premium for everything
            ("power", TaskType::Chat) => "gpt-4o".to_string(),
            ("power", TaskType::Code) => "claude-3-5-sonnet".to_string(),
            ("power", TaskType::Analysis) => "claude-3-5-sonnet".to_string(),

            // Default fallback
            _ => "gpt-4o-mini".to_string(),
        }
    }

    /// Get the cheapest available model
    fn get_cheapest_model(&self, _plan: &str) -> String {
        // Always return the cheapest model when user is near limit
        "gpt-4o-mini".to_string()
    }

    /// Check if a model is allowed for a given plan
    fn is_model_allowed_for_plan(&self, model: &str, plan: &str) -> bool {
        let allowed_models = self.get_allowed_models_for_plan(plan);
        allowed_models.contains(&model.to_string())
    }

    /// Get list of allowed models for a plan
    fn get_allowed_models_for_plan(&self, plan: &str) -> Vec<String> {
        match plan {
            "free" => vec![
                "gpt-4o-mini".to_string(),
                "claude-3-haiku".to_string(),
                "gemini-2.0-flash".to_string(),
            ],
            "starter" => vec![
                "gpt-4o-mini".to_string(),
                "gpt-3.5-turbo".to_string(),
                "claude-3-haiku".to_string(),
                "gemini-2.0-flash".to_string(),
            ],
            "pro" => vec![
                "gpt-4o-mini".to_string(),
                "gpt-4o".to_string(),
                "gpt-3.5-turbo".to_string(),
                "claude-3-haiku".to_string(),
                "claude-3-5-sonnet".to_string(),
                "gemini-2.0-flash".to_string(),
                "gemini-1.5-pro".to_string(),
            ],
            "power" => vec![
                "gpt-4o-mini".to_string(),
                "gpt-4o".to_string(),
                "gpt-4".to_string(),
                "gpt-3.5-turbo".to_string(),
                "claude-3-haiku".to_string(),
                "claude-3-5-sonnet".to_string(),
                "claude-3-sonnet".to_string(),
                "claude-3-opus".to_string(),
                "gemini-2.0-flash".to_string(),
                "gemini-1.5-pro".to_string(),
            ],
            _ => vec!["gpt-4o-mini".to_string()],
        }
    }

    // ============================================
    // TASK CLASSIFICATION (can be enhanced)
    // ============================================

    /// Classify task type from prompt (simple heuristic)
    pub fn classify_task(prompt: &str) -> TaskType {
        let prompt_lower = prompt.to_lowercase();

        if prompt_lower.contains("code")
            || prompt_lower.contains("function")
            || prompt_lower.contains("debug")
            || prompt_lower.contains("implement")
            || prompt_lower.contains("rust")
            || prompt_lower.contains("python")
            || prompt_lower.contains("javascript")
        {
            TaskType::Code
        } else if prompt_lower.contains("analyze")
            || prompt_lower.contains("explain")
            || prompt_lower.contains("summary")
        {
            TaskType::Analysis
        } else {
            TaskType::Chat
        }
    }

    // ============================================
    // MODEL METADATA
    // ============================================

    /// Get human-readable model name for display
    pub fn get_model_display_name(model: &str) -> String {
        match model {
            "gpt-4o-mini" => "GPT-4o Mini".to_string(),
            "gpt-4o" => "GPT-4o".to_string(),
            "gpt-4" => "GPT-4".to_string(),
            "gpt-3.5-turbo" => "GPT-3.5 Turbo".to_string(),
            "claude-3-5-sonnet" => "Claude 3.5 Sonnet".to_string(),
            "claude-3-haiku" => "Claude 3 Haiku".to_string(),
            "claude-3-opus" => "Claude 3 Opus".to_string(),
            "claude-3-sonnet" => "Claude 3 Sonnet".to_string(),
            "gemini-2.0-flash" => "Gemini 2.0 Flash".to_string(),
            "gemini-1.5-pro" => "Gemini 1.5 Pro".to_string(),
            _ => model.to_string(),
        }
    }

    /// Get provider from model name
    pub fn get_provider(model: &str) -> String {
        if model.starts_with("gpt-") {
            "openai".to_string()
        } else if model.starts_with("claude-") {
            "anthropic".to_string()
        } else if model.starts_with("gemini-") {
            "google".to_string()
        } else {
            "unknown".to_string()
        }
    }
}

#[derive(Debug, Clone, Copy)]
pub enum TaskType {
    Chat,     // General conversation
    Code,     // Code generation/debugging
    Analysis, // Deep analysis/explanation
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_task_classification() {
        assert!(matches!(
            ModelRouter::classify_task("Write a function to sort an array"),
            TaskType::Code
        ));
        assert!(matches!(
            ModelRouter::classify_task("Explain how this code works"),
            TaskType::Analysis
        ));
        assert!(matches!(
            ModelRouter::classify_task("Hello, how are you?"),
            TaskType::Chat
        ));
    }

    #[test]
    fn test_model_display_names() {
        assert_eq!(
            ModelRouter::get_model_display_name("gpt-4o-mini"),
            "GPT-4o Mini"
        );
        assert_eq!(
            ModelRouter::get_model_display_name("claude-3-5-sonnet"),
            "Claude 3.5 Sonnet"
        );
    }

    #[test]
    fn test_provider_detection() {
        assert_eq!(ModelRouter::get_provider("gpt-4o"), "openai");
        assert_eq!(ModelRouter::get_provider("claude-3-haiku"), "anthropic");
        assert_eq!(ModelRouter::get_provider("gemini-2.0-flash"), "google");
    }
}
