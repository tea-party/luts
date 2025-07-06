//! Simple markdown renderer for ratatui TUI
//! 
//! This module provides basic markdown rendering functionality without external dependencies
//! that might cause version conflicts.

use ratatui::{
    style::{Color, Modifier, Style},
    text::{Line, Span, Text},
};
use regex::Regex;

pub struct SimpleMarkdownRenderer {
    bold_style: Style,
    italic_style: Style,
    code_style: Style,
    heading_styles: [Style; 6],
}

impl Default for SimpleMarkdownRenderer {
    fn default() -> Self {
        Self {
            bold_style: Style::default().add_modifier(Modifier::BOLD).fg(Color::Green),
            italic_style: Style::default().add_modifier(Modifier::ITALIC).fg(Color::Yellow),
            code_style: Style::default().bg(Color::DarkGray).fg(Color::White),
            heading_styles: [
                Style::default().fg(Color::Blue).add_modifier(Modifier::BOLD),   // H1
                Style::default().fg(Color::Cyan).add_modifier(Modifier::BOLD),   // H2
                Style::default().fg(Color::Green).add_modifier(Modifier::BOLD),  // H3
                Style::default().fg(Color::Yellow).add_modifier(Modifier::BOLD), // H4
                Style::default().fg(Color::Red).add_modifier(Modifier::BOLD),    // H5
                Style::default().fg(Color::Magenta).add_modifier(Modifier::BOLD), // H6
            ],
        }
    }
}

impl SimpleMarkdownRenderer {
    pub fn render(&self, content: &str) -> Text<'static> {
        let lines: Vec<Line> = content.lines().map(|line| self.render_line(line)).collect();
        Text::from(lines)
    }

    fn render_line(&self, line: &str) -> Line<'static> {
        // Handle headings
        if let Some(heading_level) = self.parse_heading(line) {
            let text = line.trim_start_matches('#').trim();
            return Line::from(vec![
                Span::styled(
                    format!("{} {}", "#".repeat(heading_level), text),
                    self.heading_styles.get(heading_level.saturating_sub(1)).copied()
                        .unwrap_or(Style::default())
                )
            ]);
        }

        // Handle list items
        if line.trim_start().starts_with("- ") || line.trim_start().starts_with("* ") {
            let indent = line.len() - line.trim_start().len();
            let text = line.trim_start().trim_start_matches(['-', '*']).trim();
            return Line::from(vec![
                Span::raw(" ".repeat(indent)),
                Span::styled("• ", Style::default().fg(Color::Blue)),
                Span::raw(text.to_string()),
            ]);
        }

        // Handle numbered lists
        let numbered_list_re = Regex::new(r"^(\s*)(\d+)\.\s+(.*)$").unwrap();
        if let Some(captures) = numbered_list_re.captures(line) {
            let indent = captures.get(1).map_or("", |m| m.as_str());
            let number = captures.get(2).map_or("", |m| m.as_str());
            let text = captures.get(3).map_or("", |m| m.as_str());
            return Line::from(vec![
                Span::raw(indent.to_string()),
                Span::styled(format!("{}. ", number), Style::default().fg(Color::Blue)),
                Span::raw(text.to_string()),
            ]);
        }

        // Handle inline formatting
        self.render_inline_formatting(line)
    }

    fn parse_heading(&self, line: &str) -> Option<usize> {
        let trimmed = line.trim_start();
        if trimmed.starts_with('#') {
            let level = trimmed.chars().take_while(|&c| c == '#').count();
            if level <= 6 && trimmed.chars().nth(level).is_none_or(|c| c.is_whitespace()) {
                Some(level)
            } else {
                None
            }
        } else {
            None
        }
    }

    fn render_inline_formatting(&self, text: &str) -> Line<'static> {
        let mut spans = Vec::new();
        let mut current_pos = 0;
        let text = text.to_string(); // Convert to owned String for 'static lifetime

        // Regex patterns for different formatting
        let bold_re = Regex::new(r"\*\*([^*]+)\*\*").unwrap();
        let italic_re = Regex::new(r"\*([^*]+)\*").unwrap();
        let code_re = Regex::new(r"`([^`]+)`").unwrap();

        // Collect all matches with their positions
        let mut matches: Vec<(usize, usize, MatchType)> = Vec::new();

        for mat in bold_re.find_iter(&text) {
            matches.push((mat.start(), mat.end(), MatchType::Bold));
        }
        for mat in italic_re.find_iter(&text) {
            // Skip if this italic is part of a bold pattern
            if !bold_re.is_match(&text[mat.start().saturating_sub(2)..mat.end().saturating_add(2).min(text.len())]) {
                matches.push((mat.start(), mat.end(), MatchType::Italic));
            }
        }
        for mat in code_re.find_iter(&text) {
            matches.push((mat.start(), mat.end(), MatchType::Code));
        }

        // Sort matches by position
        matches.sort_by_key(|&(start, _, _)| start);

        for (start, end, match_type) in matches {
            // Add text before match
            if start > current_pos {
                spans.push(Span::raw(text[current_pos..start].to_string()));
            }

            // Add formatted match
            match match_type {
                MatchType::Bold => {
                    let inner = &text[start + 2..end - 2];
                    spans.push(Span::styled(inner.to_string(), self.bold_style));
                }
                MatchType::Italic => {
                    let inner = &text[start + 1..end - 1];
                    spans.push(Span::styled(inner.to_string(), self.italic_style));
                }
                MatchType::Code => {
                    let inner = &text[start + 1..end - 1];
                    spans.push(Span::styled(inner.to_string(), self.code_style));
                }
            }

            current_pos = end;
        }

        // Add remaining text
        if current_pos < text.len() {
            spans.push(Span::raw(text[current_pos..].to_string()));
        }

        // If no formatting was found, return the whole line as raw text
        if spans.is_empty() {
            spans.push(Span::raw(text));
        }

        Line::from(spans)
    }
}

#[derive(Debug, Clone, Copy)]
enum MatchType {
    Bold,
    Italic,
    Code,
}