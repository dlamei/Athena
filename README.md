# Langebra

Defines an intuitive syntax for writing math expressions with a keyboard

# Syntax

## short overfiew

| math notation                                                                                | Langebra                                   |
| ---                                                                                          | ---                                        |
| $${ \mathbb{N, Z, Q, R, C} }$$                                                               | ``` N, Z, Q, R, C ```                      |
| $${ \infty, \pi }$$                                                                          | ```inf, pi```                              |
| $${ a \in \mathbb{N} }$$                                                                     | ``` a: N ```                               |
| $${ (a, b) \in \mathbb{N} \times \mathbb{N} }$$                                              | ``` (a, b): N x N ```                      |
| $${ a = 1, \ a \in \mathbb{N} }$$                                                            | ``` a: N = 1 ```                           |
| $${ h = f(0) }$$                                                                             | ``` h := f(0) ```                          |
| $${ f: x \mapsto x + 1 }$$                                                                   | ``` f :: (x) -> { x + 1 } ```              |
|                                                                                              | ``` f :: x -> x + 1 ```                    |
| $${ \text{sqr}: \mathbb{R} \mapsto \mathbb{R} ; \ x \mapsto x^2  }$$                         | ``` sqr :: (x: R) -> R { x^2 }   ```       |
| $${ \text{add}: \mathbb{R} \times \mathbb{R} \mapsto \mathbb{R} ; \ (x, y) \mapsto x + y }$$ | ``` add :: (x: R, y: R) -> R { x + y } ``` |
| $${ f'(x) }$$                                                                                | ``` f'(x) ```                              |
| $${ F = \int_a^b { f(x) }\ dx }$$                                                            | ``` F :: int[a, b] { f } dx ```            |


## language features

### constants

```
G :: 1.98

// functions are always constants
f :: x -> x^2
```

predefined:

|                                |                       |
| ---                            | ---                   |
| $${ \mathbb{N, Z, Q, R, C} }$$ | ``` N, Z, Q, R, C ``` |
| $${ \pi, e}$$          | ```pi, e```      |


### range

|                    |                 |
| ---                | ---             |
| $${a \in [1, 5]}$$ | ```a: [1, 5]``` |
| $${a \in [1, 5)}$$ | ```a: [1, 5)``` |
| $${a \in (1, 5)}$$ | ```a: (1, 5)``` |

### sum, product, etc...

|                           |                            |
| ---                       | ---                        |
| $${\sum_{i = 0}^{n} i}$$  | ```sum[i = 0, n] { i }```  |
| $${\prod_{i = 0}^{n} i}$$ | ```prod[i = 0, n] { i }``` |
| $${\sqrt[n] {x} }$$       | ```root[n] { x }```        |

### derivative

|                                     |                      |
| ---                                 | ---                  |
| $${f'}$$                            | ``` f' ```           |
| $$ \frac{\partial f}{\partial x} $$ | ``` d { f } / dx ``` |

### integral

|                                       |                                  |
| ---                                   | ---                              |
| $${ F = \int { x }\ dx }$$            | ``` F :: int { x } dx```         |
| $${ F = \int_a^{\infty} { x }\ dx }$$ | ``` F :: int[a, inf] { x } dx``` |
| $${ \int {\int dx }\ dy }$$           | ``` int { int { } dx } dy ```    |

### piecewise function

$${ 
    f =
    \begin{cases} 
    ax  &\text{if} \; x \in (0, 1]\\ 
    x^2 &\text{if} \; x \in (1, \infty)
    \end{cases}
}$$

```
f :: {
    a * x if x: (0, 1],
    x^2 if x: (1, inf),
}
```

### advanced function signatures

You can use Subsets for function signatures:
```
add_one: N -> N : (x: R) -> R { x + 1 }
```

fully defined signature:
```
add : R x R -> R : (x: R, y: R) -> R { x + y } 
```

shorthand:
```
add : R x R -> R : (x, y) -> x + y 
add :: (x: R, y: R) -> R { x + y }
``` 
