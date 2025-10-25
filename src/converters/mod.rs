// This module is maintained for backward compatibility
// New code should use the providers module instead

pub use crate::providers::{LlmProvider, OpenAIProvider};

// Backward compatibility alias
pub use OpenAIProvider as OpenAIConverter;

// Backward compatibility trait alias
pub use LlmProvider as ConvertToCooklang;
