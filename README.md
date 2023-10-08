# Langebra

Defines an intuitive syntax for writing math expressions with a keyboard

## Syntax

| math notation                                                                              | Langebra                                   |
| ---                                                                                        | ---                                        |
| $$ \mathbb{N, Z, Q, R, C} $$                                                               | ``` N, Z, Q, R, C ```                      |
| $$ \infty $$                                                                               | ```inf```                                  |
| $$ a \in \mathbb{N}$$                                                                      | ``` a: N ```                               |
| $$ (a, b) \in \mathbb{N} \times \mathbb{N} $$                                              | ``` (a, b): N x N ```                      |
| $$ a = 1, \ a \in \mathbb{N} $$                                                            | ``` a: N = 1 ```                           |
| $$ h = f(0) $$                                                                             | ``` h := f(0) ```                          |
| $$ f: x \mapsto x + 1 $$                                                                   | ``` f :: (x) -> { x + 1 } ```              |
|                                                                                            | ``` f :: x -> x + 1 ```                    |
| $$ \text{sqr}: \mathbb{R} \mapsto \mathbb{R} ; \ x \mapsto x^2  $$                         | ``` sqr : R -> R : x -> x^2  ```           |
| $$ \text{add}: \mathbb{R} \times \mathbb{R} \mapsto \mathbb{R} ; \ (x, y) \mapsto x + y $$ | ``` add : R x R -> R : (x, y) -> x + y ``` |
| $$ f'(x) $$                                                                                | ``` f'(x) ```                              |
| $$ F(x) = \int_a^b { f(x) }\ dx $$                                                         | ``` F :: int[a, b] { f(x) } dx ```         |

### advanced function signatures

You can use Subsets as argument types:
```
add_one: R -> R : (x: N) -> N { x + 1 }
```

fully defined signature:
```
add : R x R -> R : (x: R, y: R) -> R { x + y } 
```

shorthand:
```
add : R x R -> R : (x, y) -> x + y 
add :: (x: R, y: R) -> R { x + y }
add :: (x: R, y: R) -> x + y // return type deduced by compiler
``` 
