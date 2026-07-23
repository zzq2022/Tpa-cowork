//! Output-language resolution and section-title localization for recap reports.
//!
//! The recap pipeline drives its LLM prompts in the user's language and writes
//! localized section titles into the persisted report (a language snapshot), so
//! both the Dashboard view and the exported HTML render in one consistent
//! language. Facet/section *prose* is produced by the LLM via a language
//! directive built from [`language_name`]; section *titles* are fixed strings
//! translated here.

use crate::{config::AppConfig, i18n::SUPPORTED_LOCALES};

/// Resolve the effective output locale for recap generation.
///
/// Precedence: explicit `recap.language` > global `AppConfig.language` >
/// system locale. Empty / `"auto"` at any level falls through to the next.
pub(super) fn effective_recap_locale(config: &AppConfig) -> String {
    crate::i18n::effective_locale(config.recap.language.as_deref(), &config.language).to_string()
}

/// Human-readable language name (with native script hint) for a locale code,
/// used to instruct the LLM which language to write in. Unknown codes fall
/// back to English.
pub(super) fn language_name(locale: &str) -> &'static str {
    crate::i18n::language_name(locale)
}

///// Localized report list title, e.g. `复盘 2024-01-01 → 2024-02-01 (12 个会话)`.
/// Uses an invariant `count + word` form to avoid per-language plural rules.
pub(super) fn report_title(locale: &str, start: &str, end: &str, sessions: u32) -> String {
    let (prefix, word) = match locale {
        "zh" => ("复盘", "个会话"),
        "zh-TW" => ("回顧", "個對話"),
        _ => ("Recap", "sessions"),
    };
    format!("{prefix} {start} → {end} ({sessions} {word})")
}

/// Column index into the per-section translation rows below. Order must match
/// the literal arrays in [`localized_section_title`]. English at index 2 is the
/// fallback for unknown locales.
fn locale_index(locale: &str) -> usize {
    // Single source of truth: position in SUPPORTED_LOCALES. Unknown → English.
    SUPPORTED_LOCALES
        .iter()
        .position(|&l| l == locale)
        .unwrap_or(2)
}

/// Localized title for a recap section `key`. Rows are ordered
/// `[zh, zh-TW, en]`. Unknown keys fall back to the English title; unknown
/// locales fall back to the English column.
pub(super) fn localized_section_title(key: &str, locale: &str) -> &'static str {
    let row: [&'static str; 3] = match key {
        "project_areas" => ["你的工作领域", "你的工作領域", "What you work on"],
        "interaction_style" => [
            "你如何使用 TPA CoWork",
            "你如何使用 TPA CoWork",
            "How you use TPA CoWork",
        ],
        "what_works" => ["哪些做得好", "哪些做得好", "What's working well"],
        "friction_analysis" => ["卡点在哪", "卡點在哪", "Where things get stuck"],
        "agent_tool_optimization" => [
            "智能体与工具优化",
            "智能體與工具最佳化",
            "Agent & tool optimization",
        ],
        "memory_skill_recommendations" => [
            "记忆与技能建议",
            "記憶與技能建議",
            "Memory & skill recommendations",
        ],
        "cost_optimization" => ["成本优化", "成本最佳化", "Cost optimization"],
        "suggestions" => ["建议", "建議", "Suggestions"],
        "on_the_horizon" => ["未来可期", "未來可期", "On the horizon"],
        "fun_ending" => ["难忘瞬间", "難忘瞬間", "Memorable moment"],
        "at_a_glance" => ["一览", "一覽", "At a glance"],
        _ => return "",
    };
    row[locale_index(locale)]
}

/// Language directive for facet-extraction prompts: natural-language fields
/// follow the locale; enum / JSON-key / category tokens stay English so
/// aggregation stays stable. Empty for English.
pub(super) fn facet_language_directive(locale: &str) -> String {
    if locale.eq_ignore_ascii_case("en") {
        return String::new();
    }
    format!(
        "IMPORTANT: Write every natural-language string value (underlyingGoal, frictionDetail, \
         primarySuccess, briefSummary, userInstructions) in {lang}. Keep all JSON keys and the \
         outcome / sessionType / goalCategories token values in English exactly as specified.",
        lang = language_name(locale)
    )
}

/// Language directive for report-section prose. Empty for English. Keeps code
/// identifiers, model names, paths and Hope Agent command names untranslated.
pub(super) fn section_language_directive(locale: &str) -> String {
    if locale.eq_ignore_ascii_case("en") {
        return String::new();
    }
    format!(
        "IMPORTANT: Write the entire section (all prose, headings, and bullet labels) in {lang}. \
         Keep code identifiers, model names, file paths, and Hope Agent command names (e.g. \
         /remember) unchanged.",
        lang = language_name(locale)
    )
}

#[cfg(test)]
mod tests {
    use super::*;

    const SECTION_KEYS: [&str; 11] = [
        "project_areas",
        "interaction_style",
        "what_works",
        "friction_analysis",
        "agent_tool_optimization",
        "memory_skill_recommendations",
        "cost_optimization",
        "suggestions",
        "on_the_horizon",
        "fun_ending",
        "at_a_glance",
    ];

    #[test]
    fn every_supported_locale_is_fully_covered() {
        for &loc in &SUPPORTED_LOCALES {
            assert_ne!(language_name(loc), "", "language_name empty for {loc}");
            assert!(
                !report_title(loc, "a", "b", 1).is_empty(),
                "report_title empty for {loc}"
            );
            for key in SECTION_KEYS {
                assert!(
                    !localized_section_title(key, loc).is_empty(),
                    "section title empty for {key}/{loc}"
                );
            }
        }
    }

    #[test]
    fn locale_columns_are_not_misaligned() {
        let expected = [
            ("zh", "你的工作领域"),
            ("zh-TW", "你的工作領域"),
            ("en", "What you work on"),
        ];
        for (loc, title) in expected {
            assert_eq!(
                localized_section_title("project_areas", loc),
                title,
                "{loc}"
            );
        }
        assert_eq!(localized_section_title("at_a_glance", "zh"), "一览");
        assert_eq!(locale_index("en"), 2);
    }

    #[test]
    fn unknown_locale_falls_back_to_english() {
        assert_eq!(locale_index("de"), 2);
        assert_eq!(
            localized_section_title("project_areas", "de"),
            "What you work on"
        );
        assert_eq!(language_name("de"), "English");
    }

    #[test]
    fn effective_locale_normalizes_case_and_unsupported() {
        let mut cfg = AppConfig::default();
        cfg.recap.language = Some("zh-tw".to_string());
        assert_eq!(effective_recap_locale(&cfg), "zh-TW");
        cfg.recap.language = Some("ZH".to_string());
        assert_eq!(effective_recap_locale(&cfg), "zh");
    }
}
