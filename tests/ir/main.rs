//! IR lowering and serialization tests (generation, round-trip, folding, compile-time evaluation).

mod compile_time_arithmetic_tests;
mod compile_time_control_flow_errors_tests;
mod compile_time_control_flow_tests;
mod compile_time_integer_overflow_tests;
mod compile_time_tests;
mod compound_assignment_folding_tests;
mod constant_propagation_tests;
mod dead_branch_elimination_tests;
mod ir_fuzzer_tests;
mod ir_generation_tests;
mod ir_roundtrip_tests;
mod phase3_ir_generation_tests;
mod phase4_qualified_names_validation;
