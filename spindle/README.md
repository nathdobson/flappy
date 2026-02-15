# Spindle

Spindle is a simple scripting language for controlling the display.

## Features

* Imperative control flow
* Dynamic typing
* Reference-counted garbage collection
* Incremental heap compaction to avoid memory fragmentation

## Types

* `i64` - a signed 64-bit integer.
* `bool` - false or true
* `null` - a type with the single value `null`
* `string` - a heap reference to a utf-8 encoded string

## Functions

* `display` - concatenates all parameters and show the resulting string on the display,
  returning once the flaps stop spinning.
  ```
  display("HelloWorld");
  ```
* `print` - print a string to the serial console.
* `sleep_ms` - wait for a certain number of milliseconds to pass.
  ```
  display("Hello");
  sleep_ms(1000);
  display("Bye");
  ```

## Language features
* String literals with double quotes: (e..g `"helloworld"`
* Integer literals (e.g. `10`)
* `null` literal
* boolean literals `false` and `true`.
* Arithmetic operators on integers `+`, `-`, `*`, `/`.
* Variables with declarations with `let` statements.
  ```
  let x = 1;
  display(x);
  ```
* `if` and `if`-`else` statements
    ```
    if foo {
        ...
    }
    ```
    ```
    if foo {
        ...
    } else {
        ....
    }
    ```
    ```
    if foo {
        ...
    } else if bar {
        ...
    } else {
        ...
    }
    ```
* `for` range loops (first index is inclusive, second index is exclusive).
    ```
    for x in 0..10 {
        print(x);
    }
    ```