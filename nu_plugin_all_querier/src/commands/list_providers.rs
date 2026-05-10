use nu_plugin::{EngineInterface, EvaluatedCall, SimplePluginCommand};
use nu_protocol::{Category, Example, LabeledError, Signature, Span, Value};

use crate::AllQuerierPlugin;

pub struct ListProviders;

impl SimplePluginCommand for ListProviders {
    type Plugin = AllQuerierPlugin;

    fn name(&self) -> &str {
        "list providers"
    }

    fn signature(&self) -> Signature {
        Signature::build(self.name()).category(Category::Experimental)
    }

    fn description(&self) -> &str {
        "List supported external provider links (e.g. pcgw, waifu)"
    }

    fn examples(&self) -> Vec<Example<'_>> {
        vec![Example {
            example: "list providers",
            description: "List all supported provider links",
            result: None,
        }]
    }

    fn run(
        &self,
        _plugin: &AllQuerierPlugin,
        _engine: &EngineInterface,
        call: &EvaluatedCall,
        _input: &Value,
    ) -> Result<Value, LabeledError> {
        let span = call.head;

        let rows = allq_providers::supported_provider_links()
            .iter()
            .map(|link| {
                let cols = vec![
                    "primaryAlias".to_string(),
                    "source".to_string(),
                    "propertyId".to_string(),
                    "supportedItemTypes".to_string(),
                    "description".to_string(),
                ];
                let vals = vec![
                    Value::string(link.primary_alias, span),
                    Value::string(link.source, span),
                    Value::string(link.property_id, span),
                    Value::list(
                        link.supported_item_types
                            .iter()
                            .map(|t| Value::string(*t, span))
                            .collect(),
                        span,
                    ),
                    Value::string(link.description, span),
                ];
                Value::record(
                    cols.into_iter().zip(vals).collect(),
                    span,
                )
            })
            .collect();

        Ok(Value::list(rows, span))
    }
}
