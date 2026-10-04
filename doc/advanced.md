<img src="lale-logo.jpg" align="right" alt="Lale logo" width="200">

# Advanced Lale Programming

This guide collects advanced techniques and idioms for Lale programmers who are already comfortable with the basics in [lale.md](lale.md).

## Visitor Pattern: Data and Operations

In object-oriented languages such as Java, C#, or C++, the **Visitor Pattern** is a common technique for keeping a data structure separate from the operations performed on it.

For example, a program that works with geometric shapes may need to:

- calculate an area,
- calculate a perimeter,
- print a description,
- export the shape to a file,
- draw it on the screen.

The classic Visitor Pattern stores the data in one place and implements each operation in a separate visitor class. Lale solves the same underlying problem with three simpler ideas:

- **enums** describe the data,
- **functions** describe the operations,
- **`switch`** dispatches on the actual data variant.

### Defining the Data

```lale
enum Shape
    Circle(f64)
    Rectangle(f64, f64)
    Triangle(f64, f64)
end enum
```

A `Shape` holds one of three variants. The enum contains only data — it knows nothing about areas, descriptions, or any other operation.

### Adding an Operation

```lale
fn area(shape as Shape) returns f64
    switch shape

    case Circle(radius):
        return 3.14159265359 ⋅ radius²

    case Rectangle(width, height):
        return width ⋅ height

    case Triangle(base, height):
        return 0.5 ⋅ base ⋅ height

    end switch
end fn
```

The function examines the shape and performs the calculation for the matching variant.

### Adding Another Operation

```lale
fn description(shape as Shape) returns text
    switch shape

    case Circle(radius):
        return "Circle with radius {radius}"

    case Rectangle(width, height):
        return "Rectangle {width} ⋅ {height}"

    case Triangle(base, height):
        return "Triangle {base} ⋅ {height}"

    end switch
end fn
```

Notice that the `Shape` enum did not change.

### Using the Functions

```lale
var shape as Shape = Circle(5.0)

write description(shape)
write "Area: {area(shape)}"
```

```text
Circle with radius 5
Area: 78.53981633975
```

### Why This Matters

Adding an operation is just adding a function — the data structure stays unchanged. Responsibilities stay clear:

- **Enums describe data.**
- **Functions perform work on that data.**

### Comparison with the Classical Visitor Pattern

The visitor pattern usually requires interfaces, virtual methods, visitor classes, and `accept()` methods. Lale reaches the same separation with ordinary functions and `switch`, which is simpler and easier to read.

### Adding a Variant Is Also Safe

The visitor pattern has a famous trade-off called the **expression problem**: adding an operation is easy, but adding a new data variant usually forces every operation to be updated.

Lale turns that trade-off into a safety net. The `switch` statement must be exhaustive over an enum, so adding a variant makes the compiler list every `switch` that needs a new case:

```text
Switch is not exhaustive: missing variant(s) 'Square' of enum 'Shape'. Add the missing case(s) or a default case.
```

That is exactly the change a developer must make, reported precisely instead of discovered at run time.

### Notes on the Example

- `⋅` (U+22C5) is Lale's dot multiplication operator; `*` works too.
- `radius²` uses a superscript digit as a power operator. This is valid in Lale
