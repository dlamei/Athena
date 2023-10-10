# Langebra

Defines an intuitive syntax for writing math expressions with a keyboard

# language features


## Primitive Types

|                  |         |
| ---              | ---     |
| $${\mathbb{B}}$$ | ```B``` |
| $${\mathbb{N}}$$ | ```N``` |
| $${\mathbb{Z}}$$ | ```Z``` |
| $${\mathbb{R}}$$ | ```R``` |
| $${\mathbb{Q}}$$ | ```Q``` |
| $${\mathbb{C}}$$ | ```C``` |
| $${\mathbb{F64}}$$ | ```F64``` |

These types act similar to primitive datatypes in other languages. B is just a boolean, N is a BigUInt, Z a BigInt and R is a BigRational.
Q on the other hand will be some symbolic expression, that can't be entierly evaluated. 
For that case we have the F64 set for all 64-bit floats.

These types are necessary for a robust syntax, but they should be mostly infered.

### assignment

```
// assign the value of 1 to a as a natural number
a: N = 1

// let the compiler deduce the variable
a := 1

// define a real number constant
G: R: 1.98
//or (deduce variable)
G :: 1.98

// define a function (functions are always constants)
f :: x -> x^2
```

### vector / matrix

|                                     |                    |
| ---                                 | ---                |
| $${\mathbb{R \times R}}$$           | ```v: R x R```     |
| $${\vec{v} \in \mathbb{R^n}}$$      | ```v: R^n```       |
| $${M \in \mathbb{R^{m \times n}}}$$ | ```M: R^(m x n)``` |

**ranges / sets:**

|                              |                  |
| ---                          | ---              |
| $${[1, 5] \sub \mathbb{N}}$$ | ```[1, 5] c N``` |
| $${[1, 5)}$$                 | ```[1, 5)```     |
| $${(1, 5)}$$                 | ```(1, 5)```     |

## sum, product, etc...

|                           |                            |
| ---                       | ---                        |
| $${\sum_{i = 0}^{n} i}$$  | ```sum[i = 0, n] { i }```  |
| $${\prod_{i = 0}^{n} i}$$ | ```prod[i = 0, n] { i }``` |
| $${\sqrt {x} }$$          | ```sqrt { x }```           |
| $${\sqrt[n] {x} }$$       | ```root[n] { x }```        |

## derivative

|                                       |                    |
| ---                                   | ---                |
| $${f'}$$                              | ``` f' ```         |
| $${f''}$$                             | ``` f" ```         |
| $${f^{k}}$$                           | ``` f'[k] ```      |
| $${\partial x}$$                      | ```d[x]```         |
| $${ \frac{\partial f}{\partial x} }$$ | ``` d[f] / d[x]``` |

## integral

|                                       |                                      |
| ---                                   | ---                                  |
| $${ F = \int { x }\ dx }$$            | ``` F :: int { x } d[x]```           |
| $${ F = \int_a^{\infty} { x }\ dx }$$ | ``` F :: int[a, inf] { x } d[x]```   |
| $${ \int {\int dx }\ dy }$$           | ``` int { int { .. } d[x] } d[y] ``` |

TODO: for now the derivative and integral uses the same domain and range as the original function. This just means that some result could be inf or NaN

## piecewise function

$${ 
    f =
    \begin{cases} 
    ax  &\text{if} \; x \in [0, 1]\\ 
    x^2 &\text{if} \; x \in (1, \infty)
    \end{cases}
}$$

```
f :: PW {
    a * x if x in [0, 1),
    x^2   if x in [1, inf),
}
```

## advanced function signatures

fully defined signature:
```
add : R x R -> R : (x: R, y: R) -> R { x + y } 
```

the following are equivalent:
```
add : R x R -> R : (x, y) -> x + y 
add :: (x: R, y: R) -> R { x + y }
```
You can use Subsets for the domain and ranges of the function for the signature:
```
add_one: N -> N : (x: R) -> R { x + 1 }
``` 

TODO: how should the domain / range of the function be treated when integrated / derived?

## cheatsheet

| math notation                                                                                | Langebra                                   |
| ---                                                                                          | ---                                        |
| $${ \mathbb{N, Z, Q, R, C} }$$                                                               | ``` N, Z, Q, R, C ```                      |
| $${ \mathbb{R} \times \mathbb{R} }$$                                                         | ```R x R``` or ```R^2```                   |
| $${ \infty, \pi }$$                                                                          | ```inf, pi```                              |
| $${ a \in \mathbb{N} }$$                                                                     | ``` a: N ```  or ```a in N```              |
| $${ (a, b) \in \mathbb{N} \times \mathbb{N} }$$                                              | ``` (a, b): N x N ```                      |
| $${ a = 1, \ a \in \mathbb{N} }$$                                                            | ``` a: N = 1 ```                           |
| $${ h = f(0) }$$                                                                             | ``` h := f(0) ```                          |
| $${ f: x \mapsto x + 1 }$$                                                                   | ``` f :: (x) -> { x + 1 } ```              |
|                                                                                              | ``` f :: x -> x + 1 ```                    |
| $${ \text{sqr}: \mathbb{R} \mapsto \mathbb{R} ; \ x \mapsto x^2  }$$                         | ``` sqr :: (x: R) -> R { x^2 }   ```       |
| $${ \text{add}: \mathbb{R} \times \mathbb{R} \mapsto \mathbb{R} ; \ (x, y) \mapsto x + y }$$ | ``` add :: (x: R, y: R) -> R { x + y } ``` |
| $${ f'(x) }$$                                                                                | ``` f'(x) ```                              |
| $${ F = \int_a^b { f(x) }\ dx }$$                                                            | ``` F :: int[a, b] { f } dx ```            |

