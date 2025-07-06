//! Calculator tool for AI assistants
//!
//! This module provides a simple calculator tool that can evaluate mathematical expressions.

use crate::tools::AiTool;
use anyhow::{Error, anyhow};
use async_trait::async_trait;
use serde_json::Value;

/// A simple calculator tool for evaluating mathematical expressions
pub struct MathTool;

#[async_trait]
impl AiTool for MathTool {
    fn name(&self) -> &str {
        "calculator"
    }

    fn description(&self) -> &str {
        "Evaluates mathematical expressions"
    }

    fn schema(&self) -> Value {
        serde_json::json!({
            "type": "object",
            "properties": {
                "expression": {
                    "type": "string",
                    "description": "The mathematical expression to evaluate"
                }
            },
            "required": ["expression"]
        })
    }

    async fn execute(&self, params: Value) -> Result<Value, Error> {
        self.validate_params(&params)?;

        let expression = params["expression"]
            .as_str()
            .ok_or_else(|| anyhow!("Missing 'expression' parameter"))?;

        // Use a simple evaluation approach for basic arithmetic
        // This is a very simplistic implementation that only handles basic operations
        let result = evaluate_expression(expression)?;

        Ok(Value::Number(
            serde_json::Number::from_f64(result).expect("f64 is valid serde_json::Number"),
        ))
    }

    fn validate_params(&self, params: &Value) -> Result<(), Error> {
        if !params.is_object() {
            return Err(anyhow!("Parameters must be an object"));
        }

        if !params.get("expression").is_some_and(|v| v.is_string()) {
            return Err(anyhow!("Missing or invalid 'expression' parameter"));
        }

        Ok(())
    }
}

/// Evaluate a simple mathematical expression
///
/// This is a very basic implementation that only supports +, -, *, and / operations
/// with proper precedence. It doesn't support parentheses, functions, or other advanced features.
fn evaluate_expression(expr: &str) -> Result<f64, Error> {
    // Remove all whitespace
    let expr = expr.replace(" ", "");

    // Start with addition and subtraction
    let mut result = 0.0;
    let mut current_term = 0.0;
    let mut current_op = '+';

    let mut i = 0;
    while i < expr.len() {
        // Get the next term (number or multiplication/division expression)
        let (next_i, term) = evaluate_term(&expr[i..])?;
        i += next_i;

        // Apply the current operator
        match current_op {
            '+' => current_term += term,
            '-' => current_term -= term,
            _ => return Err(anyhow!("Invalid operator: {}", current_op)),
        }

        // If we've reached the end or the next character is + or -, add the current term to the result
        if i >= expr.len() || expr.chars().nth(i) == Some('+') || expr.chars().nth(i) == Some('-') {
            result += current_term;
            current_term = 0.0;
        }

        // Get the next operator if there is one
        if i < expr.len() {
            current_op = expr.chars().nth(i).unwrap();
            i += 1;
        }
    }

    Ok(result)
}

/// Evaluate a term (number or multiplication/division expression)
fn evaluate_term(expr: &str) -> Result<(usize, f64), Error> {
    let mut current_factor = 0.0;
    let mut current_op = '*';

    let mut i = 0;
    let mut first = true;

    while i < expr.len() {
        // Get the next factor (number)
        let (next_i, factor) = evaluate_factor(&expr[i..])?;
        i += next_i;

        if first {
            current_factor = factor;
            first = false;
        } else {
            // Apply the current operator
            match current_op {
                '*' => current_factor *= factor,
                '/' => {
                    if factor == 0.0 {
                        return Err(anyhow!("Division by zero"));
                    }
                    current_factor /= factor;
                }
                _ => return Err(anyhow!("Invalid operator: {}", current_op)),
            }
        }

        // If we've reached the end or the next character is not * or /, return the result
        if i >= expr.len() || (expr.chars().nth(i) != Some('*') && expr.chars().nth(i) != Some('/'))
        {
            return Ok((i, current_factor));
        }

        // Get the next operator
        current_op = expr.chars().nth(i).unwrap();
        i += 1;
    }

    Ok((i, current_factor))
}

/// Evaluate a factor (number)
fn evaluate_factor(expr: &str) -> Result<(usize, f64), Error> {
    let mut i = 0;
    let mut num_str = String::new();
    let mut has_decimal = false;

    // Check for negative number
    if expr.starts_with('-') {
        num_str.push('-');
        i += 1;
    }

    // Parse the number
    while i < expr.len() {
        let c = expr.chars().nth(i).unwrap();
        if c.is_ascii_digit() {
            num_str.push(c);
            i += 1;
        } else if c == '.' && !has_decimal {
            num_str.push(c);
            has_decimal = true;
            i += 1;
        } else {
            break;
        }
    }

    // Convert to f64
    let num = num_str
        .parse::<f64>()
        .map_err(|_| anyhow!("Invalid number: {}", num_str))?;

    Ok((i, num))
}

#[cfg(test)]
mod tests {
    use super::*;
    use serde_json::json;

    #[test]
    fn test_evaluate_expression() {
        assert_eq!(evaluate_expression("2 + 3").unwrap(), 5.0);
        assert_eq!(evaluate_expression("2 - 3").unwrap(), -1.0);
        assert_eq!(evaluate_expression("2 * 3").unwrap(), 6.0);
        assert_eq!(evaluate_expression("6 / 3").unwrap(), 2.0);
        assert_eq!(evaluate_expression("2 + 3 * 4").unwrap(), 14.0);
        assert_eq!(
            evaluate_expression("(2 + 3) * 4").unwrap_err().to_string(),
            "Invalid number: "
        );
        assert_eq!(
            evaluate_expression("6 / 0").unwrap_err().to_string(),
            "Division by zero"
        );
    }

    #[tokio::test]
    async fn test_math_tool() {
        let tool = MathTool;

        // Test basic addition
        let params = json!({"expression": "2 + 3"});
        let result = tool.execute(params).await.unwrap();
        assert_eq!(result.as_f64().unwrap(), 5.0);

        // Test more complex expression
        let params = json!({"expression": "2 + 3 * 4"});
        let result = tool.execute(params).await.unwrap();
        assert_eq!(result.as_f64().unwrap(), 14.0);

        // Test invalid expression
        let params = json!({"expression": "2 + + 3"});
        assert!(tool.execute(params).await.is_err());

        // Test missing parameter
        let params = json!({});
        assert!(tool.execute(params).await.is_err());
    }

    #[tokio::test]
    async fn test_basic_arithmetic() {
        let tool = MathTool;
        
        // Addition
        let result = tool.execute(json!({"expression": "2 + 3"})).await.unwrap();
        assert_eq!(result.as_f64().unwrap(), 5.0);
        
        // Subtraction
        let result = tool.execute(json!({"expression": "10 - 4"})).await.unwrap();
        assert_eq!(result.as_f64().unwrap(), 6.0);
        
        // Multiplication
        let result = tool.execute(json!({"expression": "3 * 7"})).await.unwrap();
        assert_eq!(result.as_f64().unwrap(), 21.0);
        
        // Division
        let result = tool.execute(json!({"expression": "15 / 3"})).await.unwrap();
        assert_eq!(result.as_f64().unwrap(), 5.0);
    }

    #[tokio::test]
    async fn test_complex_expressions() {
        let tool = MathTool;
        
        // Order of operations
        let result = tool.execute(json!({"expression": "2 + 3 * 4"})).await.unwrap();
        assert_eq!(result.as_f64().unwrap(), 14.0);
        
        // Decimal operations
        let result = tool.execute(json!({"expression": "3.14 * 2"})).await.unwrap();
        assert!((result.as_f64().unwrap() - 6.28).abs() < 0.001);
    }

    #[tokio::test]
    async fn test_invalid_expressions() {
        let tool = MathTool;
        
        // Division by zero
        let result = tool.execute(json!({"expression": "5 / 0"})).await;
        assert!(result.is_err());
        
        // Invalid syntax
        let result = tool.execute(json!({"expression": "2 + + 3"})).await;
        assert!(result.is_err());
        
        // Empty expression evaluates to 0 in current implementation
        let result = tool.execute(json!({"expression": ""})).await.unwrap();
        assert_eq!(result.as_f64().unwrap(), 0.0);
    }

    #[tokio::test]
    async fn test_parameter_validation() {
        let tool = MathTool;
        
        // Missing expression parameter
        let result = tool.execute(json!({})).await;
        assert!(result.is_err());
        
        // Wrong parameter type
        let result = tool.execute(json!({"expression": 123})).await;
        assert!(result.is_err());
        
        // Extra parameters (should be ignored)
        let result = tool.execute(json!({
            "expression": "2 + 2",
            "extra_param": "ignored"
        })).await.unwrap();
        assert_eq!(result.as_f64().unwrap(), 4.0);
    }

    #[tokio::test]
    async fn test_edge_cases() {
        let tool = MathTool;
        
        // Negative numbers
        let result = tool.execute(json!({"expression": "-5 + 3"})).await.unwrap();
        assert_eq!(result.as_f64().unwrap(), -2.0);
        
        // Zero operations
        let result = tool.execute(json!({"expression": "0 * 100"})).await.unwrap();
        assert_eq!(result.as_f64().unwrap(), 0.0);
    }

    #[tokio::test]
    async fn test_whitespace_handling() {
        let tool = MathTool;
        
        // Extra whitespace
        let result = tool.execute(json!({"expression": "  2   +   3  "})).await.unwrap();
        assert_eq!(result.as_f64().unwrap(), 5.0);
        
        // No whitespace
        let result = tool.execute(json!({"expression": "2+3*4"})).await.unwrap();
        assert_eq!(result.as_f64().unwrap(), 14.0);
    }

    #[test]
    fn test_tool_metadata() {
        let tool = MathTool;
        
        assert_eq!(tool.name(), "calculator");
        assert!(!tool.description().is_empty());
        
        let schema = tool.schema();
        assert!(schema["type"].as_str() == Some("object"));
        assert!(schema["properties"]["expression"].is_object());
        assert!(schema["required"].as_array().unwrap().contains(&json!("expression")));
    }
}
