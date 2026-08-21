# Testing the unit compatibility

## Negative Examples (The "Hall of Shame")

These snippets illustrate what actually causes the Lale compiler to throw an error:

```lale
// 1. CONFLICT: Declared unit does not match inferred unit
var dist as f64 in <m> = 10.0 <s> // ERROR: Initializer unit <s> doesn't match declared <m>

// 2. INCOMPATIBLE ARITHMETIC: Adding two variables with different inferred units
var length = 10.0 <m>
var time = 2.0 <s>
var result = length + time // ERROR: Addition unit mismatch (<m> vs <s>)

// 3. COMPARISON MISMATCH:
if 10.0 <kg> > 5.0 <m> // ERROR: Comparison unit mismatch

// 4. UNIT STRIPPING ATTEMPT:
var distance as f64 in <m> = 100.0
var raw_val as f64 = 0.0
raw_val = distance // ERROR: Assignment unit mismatch (unitless vs <m>)
                   // (Because raw_val was established as unitless at its definition)

// 5. SEMANTIC VIOLATION: Exponent must be unitless
var base = 10.0 <m>
var pwr = 2.0 <m>
var area = base ^ pwr // ERROR: Power operation exponent has units

// 6. SAFETY VIOLATION: Unsafe cast on dimensional values
var height = 1.8 <m>
var bits = unsafe value at (pointer to height) unsafe cast as u64 // ERROR: Unsafe cast requires unitless source
```

---

### The "Unit Stress Test" Lale File

This file is updated to ensure it tests **normalization** in valid code and correctly identifies **mismatches** in invalid code.

```lale
// ============================================================================
// LALE UNIT NORMALIZATION STRESS TEST
// ============================================================================

// Identity function to test unit propagation across calls
fn identity_joule(val as f64 in <J>) returns f64 in <kg⋅m²/s²>
    return val // Should pass: J and kg⋅m²/s² are identical
end fn

fn test_inference() returns bool
    // Inference Test
    var a = 10.0 <kg⋅m/s²> // a infers <N>
    var b as f64 in <N> = a // Valid: inferred matches declared

    // Complex Inference Chain
    var p = 100.0 <Pa>      // Pa
    var v = 0.5 <m³>        // m^3
    var energy = p * v      // energy infers <Pa⋅m³> which is <J>

    return energy == 50.0 <J>
end fn

// ============================================================================
// TEST SUITE: BENDING THE NOTATION
// ============================================================================

test suite advanced_normalization_identities

    test case the_joule_identity
        // Energy in 4 different notations
        var e1 = 1.0 <J>
        var e2 = 1.0 <N⋅m>
        var e3 = 1.0 <W⋅s>
        var e4 = 1.0 <Pa⋅m³>

        // Complex electrical energy: C * V = (A*s) * (W/A) = W*s = J
        var charge = 2.0 <C>
        var voltage = 5.0 <V>
        var e5 = charge * voltage

        assert e1 == e2
        assert e2 == e3
        assert e3 == e4
        assert e5 == 10.0 <J>
    end test case

    test case order_and_symbol_independence
        // Testing that the compiler doesn't care about order or symbol choice
        var g1 = 9.8 <m/s²>
        var g2 = 9.8 <s⁻²⋅m>    // Swapped order, superscript
        var g3 = 9.8 <m*s^-2>   // Asterisk, caret
        var g4 = 9.8 <m÷s²>     // Division sign

        assert g1 == g2
        assert g2 == g3
        assert g3 == g4
    end test case

    test case vector_to_scalar_unit_reduction
        // Force (vec3) dot Displacement (vec3) = Work (f64)
        var F⃗ as vec3 of f64 in <N> = vec3(5.0, 0.0, 0.0)
        var d⃗ as vec3 of f64 in <m> = vec3(2.0, 0.0, 0.0)

        var work = F⃗ dot d⃗ // should infer <N⋅m> -> <J>

        assert work == 10.0 <J>
    end test case

    test case dimensionless_interaction
        var ratio = 10.0 <m> / 2.0 <m> // Result is unitless
        var scale = 5.0

        // A unitless result should be comparable/assignable to dimensionless vars
        assert ratio == scale

        var angle = 1.5 <rad>
        var s = sin(angle) // sin returns unitless
        assert s <= 1.0
    end test case

end test suite

// ============================================================================
// APPENDIX: KNOWN COMPILE-TIME FAILURES
// ============================================================================
// The following block contains code that MUST be rejected by the compiler.
// They are commented out here so the rest of the file can run.

/*
fn trigger_errors()
    // 1. Unit mismatch in assignment
    var x as f64 in <m> = 1.0 <kg>

    // 2. Illegal unit stripping (inference prevents this unless established)
    var length = 5.0 <m>
    var raw as f64 = 10.0 // raw is unitless
    raw = length          // ERROR: Conflict between unitless and <m>

    // 3. Mathematical impossibility
    var bad_math = 5.0 <m> ^ 2.0 <kg> // ERROR: Exponent must be unitless

    // 4. Incompatible addition
    var oops = 1.0 <V> + 1.0 <A> // ERROR: Cannot add Volts and Amperes
end fn
*/
```

### Why this is a challenge for the compiler

1. **Semantic Aliasing:** The compiler must know that `Pa * m³` simplifies to `kg * m² * s⁻²`. This requires a "Unit Simplifier" that reduces all expressions to a canonical prime-base vector (the 7 SI base units + Radian).
2. **Zero-Exponents:** If a user writes `<kg⋅m/m>`, the compiler must normalize this to `<kg>`, effectively treating `m^0` as non-existent.
3. **The inference tracking:** The compiler must track that `var energy = p * v` results in a variable with a specific unit-bit-mask, and ensure that later in the same function, if `energy` is used, its units are still enforced.
4. **Vector Dot/Cross Compatibility:** It checks that the vector's `inner_type` unit propagation is consistent with scalar operations.
