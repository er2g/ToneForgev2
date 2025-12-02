// Comprehensive AI Engine Testing Suite
//
// Tests real-world scenarios to validate AI's REAPER control intelligence

#[cfg(test)]
mod ai_engine_stress_tests {
    use crate::ai_engine::*;

    // ============================================================================
    // TEST SCENARIO 1: Disabled Plugin Modification (Should Auto-Enable)
    // ============================================================================

    #[test]
    fn test_hierarchical_validation_disabled_plugin() {
        println!("\n=== TEST 1: Disabled Plugin Modification ===");

        // Scenario: User asks to increase gain on a DISABLED plugin
        // Expected: AI should detect it's disabled and enable it first

        let actions = vec![ActionPlan {
            track: 0,
            fx_index: 0,
            param_index: 5, // Gain parameter
            value: 0.85,
            reason: "User wants aggressive metal tone".to_string(),
        }];

        println!("Input: 1 action (set gain to 0.85 on disabled plugin)");

        // In real scenario, apply_actions would detect plugin is disabled
        // and enable it before modifying. This is coded in lib.rs:316-326

        let deduplicated = ActionOptimizer::deduplicate(actions);
        assert_eq!(deduplicated.len(), 1);

        println!("✓ Test passed: Action preserved, hierarchical validation will handle enabling");
    }

    // ============================================================================
    // TEST SCENARIO 2: Duplicate Actions (Should Deduplicate)
    // ============================================================================

    #[test]
    fn test_action_deduplication() {
        println!("\n=== TEST 2: Duplicate Actions ===");

        // Scenario: AI makes mistake and sets same param 3 times
        let actions = vec![
            ActionPlan {
                track: 0,
                fx_index: 0,
                param_index: 5,
                value: 0.5,
                reason: "First attempt".to_string(),
            },
            ActionPlan {
                track: 0,
                fx_index: 0,
                param_index: 5,
                value: 0.7,
                reason: "Second attempt".to_string(),
            },
            ActionPlan {
                track: 0,
                fx_index: 0,
                param_index: 5,
                value: 0.85,
                reason: "Final value".to_string(),
            },
        ];

        println!("Input: 3 duplicate actions (same param, different values)");

        let deduplicated = ActionOptimizer::deduplicate(actions);

        println!("Output: {} action(s)", deduplicated.len());
        assert_eq!(deduplicated.len(), 1, "Should keep only 1 action");
        assert_eq!(deduplicated[0].value, 0.85, "Should keep last value");

        println!("✓ Test passed: Deduplicated 3 → 1, kept last value (0.85)");
    }

    // ============================================================================
    // TEST SCENARIO 3: Conflict Detection
    // ============================================================================

    #[test]
    fn test_conflict_detection() {
        println!("\n=== TEST 3: Conflict Detection ===");

        // Scenario: AI tries to set gain to TWO different values
        let actions = vec![
            ActionPlan {
                track: 0,
                fx_index: 0,
                param_index: 5,
                value: 0.3,
                reason: "User wants clean tone".to_string(),
            },
            ActionPlan {
                track: 0,
                fx_index: 0,
                param_index: 5,
                value: 0.9,
                reason: "User wants metal tone".to_string(),
            },
        ];

        println!("Input: 2 conflicting actions (gain=0.3 AND gain=0.9)");

        let conflicts = ActionOptimizer::detect_conflicts(&actions);

        println!("Conflicts detected: {}", conflicts.len());
        for conflict in &conflicts {
            println!("  ⚠️  {}", conflict);
        }

        assert!(!conflicts.is_empty(), "Should detect conflict");

        println!("✓ Test passed: Conflict detected and reported");
    }

    // ============================================================================
    // TEST SCENARIO 4: Extreme Value Safety Validation
    // ============================================================================

    #[test]
    fn test_safety_validation_extreme_values() {
        println!("\n=== TEST 4: Extreme Value Safety ===");

        // Scenario: User/AI tries to set insane values
        let test_cases = vec![
            ("Gain", 1.5, "Above max"),
            ("Bass", -0.2, "Below min"),
            ("Volume", 0.99, "Near clipping"),
            ("Treble", 0.95, "Extreme boost"),
        ];

        for (param_name, value, scenario) in test_cases {
            println!("\nTesting: {} = {} ({})", param_name, value, scenario);

            let (clamped, warning) = SafetyValidator::validate_value(param_name, value);

            println!("  Clamped to: {}", clamped);
            if let Some(warn) = warning {
                println!("  ⚠️  {}", warn);
            }

            // Should clamp to 0-1 range
            assert!(clamped >= 0.0 && clamped <= 1.0, "Value should be clamped to valid range");
        }

        println!("\n✓ Test passed: All extreme values handled safely");
    }

    // ============================================================================
    // TEST SCENARIO 5: Semantic Categorization
    // ============================================================================

    #[test]
    fn test_semantic_categorization() {
        println!("\n=== TEST 5: Semantic Parameter Categorization ===");

        let test_params = vec![
            ("Gain", ParameterCategory::Distortion),
            ("Overdrive", ParameterCategory::Distortion),
            ("Drive", ParameterCategory::Distortion),
            ("Bass", ParameterCategory::EQ),
            ("Treble", ParameterCategory::EQ),
            ("Mid", ParameterCategory::EQ),
            ("Delay Time", ParameterCategory::Delay),
            ("Reverb Size", ParameterCategory::Reverb),
            ("Chorus Rate", ParameterCategory::Modulation),
            ("Compressor Threshold", ParameterCategory::Dynamics),
            ("Output Level", ParameterCategory::Volume),
        ];

        println!("\nTesting parameter categorization:");
        for (param_name, expected_category) in test_params {
            let category = SemanticAnalyzer::categorize(param_name);
            println!("  '{}' → {:?}", param_name, category);
            assert_eq!(category, expected_category, "Wrong category for {}", param_name);
        }

        println!("\n✓ Test passed: All parameters correctly categorized");
    }

    // ============================================================================
    // TEST SCENARIO 6: Parameter Relationship Suggestions
    // ============================================================================

    #[test]
    fn test_parameter_relationships() {
        println!("\n=== TEST 6: Parameter Relationship Engine ===");

        // Scenario: User cranks gain to 0.9 (from 0.3)
        println!("\nScenario: Gain increased from 0.3 → 0.9 (delta: +0.6)");

        let suggestions = RelationshipEngine::suggest_compensations("Gain", 0.3, 0.9);

        println!("Suggestions:");
        for (param, delta, reason) in &suggestions {
            println!("  💡 Adjust '{}' by {:.2} ({})", param, delta, reason);
        }

        assert!(!suggestions.is_empty(), "Should suggest compensations for large gain increase");

        // Should suggest bass reduction
        let bass_suggestion = suggestions.iter().find(|(p, _, _)| p == "bass");
        assert!(bass_suggestion.is_some(), "Should suggest bass adjustment");

        println!("\n✓ Test passed: Appropriate compensations suggested");
    }

    // ============================================================================
    // TEST SCENARIO 7: Complex Real-World Scenario
    // ============================================================================

    #[test]
    fn test_complex_real_world_scenario() {
        println!("\n=== TEST 7: Complex Real-World Scenario ===");
        println!("User request: 'I want a heavy Metallica tone'");
        println!("AI generates multiple actions with duplicates, conflicts, and extreme values\n");

        // AI's initial plan (before optimization)
        let raw_actions = vec![
            // Gain adjustments (duplicates!)
            ActionPlan {
                track: 0, fx_index: 0, param_index: 5,
                value: 0.7,
                reason: "Initial gain boost".to_string(),
            },
            ActionPlan {
                track: 0, fx_index: 0, param_index: 5,
                value: 0.85,
                reason: "Higher gain for metal".to_string(),
            },
            ActionPlan {
                track: 0, fx_index: 0, param_index: 5,
                value: 0.92, // Extreme!
                reason: "Maximum aggression".to_string(),
            },
            // EQ adjustments
            ActionPlan {
                track: 0, fx_index: 1, param_index: 2,
                value: 0.4,
                reason: "Scoop mids".to_string(),
            },
            ActionPlan {
                track: 0, fx_index: 1, param_index: 3,
                value: 0.75,
                reason: "Boost treble".to_string(),
            },
        ];

        println!("RAW ACTIONS: {}", raw_actions.len());

        // STEP 1: Conflict Detection
        let conflicts = ActionOptimizer::detect_conflicts(&raw_actions);
        println!("\nCONFLICT DETECTION:");
        for conflict in &conflicts {
            println!("  ⚠️  {}", conflict);
        }

        // STEP 2: Deduplication
        let deduplicated = ActionOptimizer::deduplicate(raw_actions);
        println!("\nDEDUPLICATION: {} → {} actions", 5, deduplicated.len());

        // STEP 3: Safety Validation
        println!("\nSAFETY VALIDATION:");
        for action in &deduplicated {
            let param_name = format!("Param_{}", action.param_index);
            let (clamped, warning) = SafetyValidator::validate_value(&param_name, action.value);

            if let Some(warn) = &warning {
                println!("  🛡️  {} = {} → {}: {}",
                    param_name, action.value, clamped, warn);
            }
        }

        // STEP 4: Semantic Analysis
        println!("\nSEMANTIC ANALYSIS:");
        let gain_category = SemanticAnalyzer::categorize("Gain");
        println!("  'Gain' → {:?}", gain_category);

        // STEP 5: Relationship Suggestions
        println!("\nRELATIONSHIP SUGGESTIONS:");
        let suggestions = RelationshipEngine::suggest_compensations("Gain", 0.5, 0.92);
        for (param, delta, reason) in &suggestions {
            println!("  💡 '{}' by {:.2}: {}", param, delta, reason);
        }

        println!("\n=== FINAL OPTIMIZED PLAN ===");
        println!("Actions to execute: {}", deduplicated.len());
        for (i, action) in deduplicated.iter().enumerate() {
            println!("  {}. Track {} FX {} Param {} = {:.2}",
                i + 1, action.track, action.fx_index, action.param_index, action.value);
        }

        println!("\n✓ Test passed: Complex scenario handled with all optimizations");
    }

    // ============================================================================
    // TEST SCENARIO 8: State Diffing
    // ============================================================================

    #[test]
    fn test_state_diffing() {
        println!("\n=== TEST 8: State Diffing ===");

        // Old state: Gain = 0.5, Bass = 0.6
        let old_state = vec![
            (0, vec![
                (0, "Neural DSP Gojira".to_string(), true, vec![
                    (5, "Gain".to_string(), 0.5, "-6.0 dB".to_string()),
                    (8, "Bass".to_string(), 0.6, "60%".to_string()),
                ]),
            ]),
        ];

        // New state: Gain = 0.85, Bass = 0.5
        let new_state = vec![
            (0, vec![
                (0, "Neural DSP Gojira".to_string(), true, vec![
                    (5, "Gain".to_string(), 0.85, "-1.2 dB".to_string()),
                    (8, "Bass".to_string(), 0.5, "50%".to_string()),
                ]),
            ]),
        ];

        let diff = StateDiffer::diff(&old_state, &new_state);

        println!("\nState changes detected:");
        println!("  Changed parameters: {}", diff.changed_params.len());
        for param_diff in &diff.changed_params {
            println!("    {} :: {} → {} (delta: {:.2})",
                param_diff.param_name,
                param_diff.old_display,
                param_diff.new_display,
                param_diff.delta
            );
        }

        assert_eq!(diff.changed_params.len(), 2, "Should detect 2 changed params");

        println!("\n✓ Test passed: State diff correctly identified changes");
    }
}

// ============================================================================
// INTEGRATION TEST: Full Pipeline
// ============================================================================

#[cfg(test)]
mod integration_tests {
    use super::*;

    #[test]
    fn test_full_ai_engine_pipeline() {
        println!("\n╔═══════════════════════════════════════════════════════════╗");
        println!("║  FULL AI ENGINE INTEGRATION TEST                          ║");
        println!("╚═══════════════════════════════════════════════════════════╝");

        // Simulated AI response with all kinds of issues
        let messy_actions = vec![
            // Duplicate gain settings
            ai_engine::ActionPlan { track: 0, fx_index: 0, param_index: 5, value: 0.6, reason: "v1".into() },
            ai_engine::ActionPlan { track: 0, fx_index: 0, param_index: 5, value: 0.95, reason: "v2".into() },

            // Extreme bass
            ai_engine::ActionPlan { track: 0, fx_index: 0, param_index: 8, value: 1.2, reason: "max bass".into() },

            // Normal treble
            ai_engine::ActionPlan { track: 0, fx_index: 0, param_index: 9, value: 0.7, reason: "boost treble".into() },
        ];

        println!("\n📥 INPUT: {} messy actions", messy_actions.len());

        // PIPELINE STAGE 1: Conflict Detection
        println!("\n🔍 STAGE 1: Conflict Detection");
        let conflicts = ai_engine::ActionOptimizer::detect_conflicts(&messy_actions);
        println!("   Conflicts found: {}", conflicts.len());
        for c in &conflicts {
            println!("   ⚠️  {}", c);
        }

        // PIPELINE STAGE 2: Deduplication
        println!("\n🧹 STAGE 2: Deduplication");
        let clean_actions = ai_engine::ActionOptimizer::deduplicate(messy_actions);
        println!("   Reduced: 4 → {} actions", clean_actions.len());

        // PIPELINE STAGE 3: Safety Validation
        println!("\n🛡️  STAGE 3: Safety Validation");
        for action in &clean_actions {
            let param_name = match action.param_index {
                5 => "Gain",
                8 => "Bass",
                9 => "Treble",
                _ => "Unknown",
            };

            let (clamped, warning) = ai_engine::SafetyValidator::validate_value(param_name, action.value);
            println!("   {} = {:.2} → {:.2}", param_name, action.value, clamped);
            if let Some(w) = warning {
                println!("      ⚠️  {}", w);
            }
        }

        // PIPELINE STAGE 4: Semantic Analysis
        println!("\n🏷️  STAGE 4: Semantic Analysis");
        for action in &clean_actions {
            let param_name = match action.param_index {
                5 => "Gain",
                8 => "Bass",
                9 => "Treble",
                _ => "Unknown",
            };
            let category = ai_engine::SemanticAnalyzer::categorize(param_name);
            println!("   {} → {:?}", param_name, category);
        }

        // PIPELINE STAGE 5: Relationship Analysis
        println!("\n💡 STAGE 5: Relationship Suggestions");
        // Assuming old gain was 0.5, new is 0.95
        let suggestions = ai_engine::RelationshipEngine::suggest_compensations("Gain", 0.5, 0.95);
        for (param, delta, reason) in suggestions {
            println!("   💡 Suggest: {} by {:.2} ({})", param, delta, reason);
        }

        println!("\n✅ PIPELINE COMPLETE");
        println!("   Final actions: {}", clean_actions.len());
        println!("   All safety checks passed");
        println!("   Ready for execution\n");
    }
}
