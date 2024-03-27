# Athena (WIP)

Intuitive math syntax for a user-friendly [CAS](https://en.wikipedia.org/wiki/Computer_algebra_system)

[web-interface](https://dlamei.github.io/Athena/)
[backend](https://github.com/dlamei/Calcu-rs)

```
// row vector
a: R^(1x3) = (1 2 3)

// column vector
a: R^(3x1) = (1 2 3)^T

a: R^3 = (1 2 3)^T
a := (1; 2; 3)
a[i: 1..3] := i
a[i: 1..3]: R^3 = i

M := ( 1 2 3; 2 4 6; 3 6 9)
M := (
    1 2 3
    2 4 6
    3 6 9
)

M[i: 1..3, j: 1..3] := i * j

_M[i: 1..3, j: 1..3] := 2 * M[i, j]

monomial :: n: N -> sum k: 0..n  { x^k }

f :: x^2 + 2x + 3
f :: x -> x^2 + 2x + 3

f :: sin(x) + y
f :: (x, y) -> sin(x) + y
f :: (x, y) -> sin(x) + y
f :: (x: R, y: R) -> sin(x) + y
f :: (x: R, y: R) -> R { sin(x) + y }

F :: int { f } d(x)
F :: int { f } d_x
F :: int a..b { f } d(x)

F :: int { x -> x^2 + 2x + 3 } d_x
F :: int { x^2 + 2x + 3 } d_x

f :: d_F / d_x
f :: d(F) / d(x)
f :: F'


J: R^(2x2) = jacobi(F)

d_x := J[0, ..]

// WIP

Even :: { x: R | x % 2 == 0 }

Even subset R

SquarePoly :: { (x) :: a*x^2 + b*x + c | a: R, b: R, c: R }

n: Even := 4
_n: R := n // possible, because Even is a subset

q(x: R) : SquarePoly : 3x^2 + 3x + 4

p(x: R) :: 3x^2 + 3x + 8

check(p in SquarePoly)
check(q in SquarePoly)


s1 := { x | p(x) == 0, x: R }

s2 := {x | p'(x) == 0, x: R }
s3 := {x | d_p(x) / d_x == 0, x: R }

check(s2 == s3)

```

<sub> inspired by GeoGebra, Desmos, SymPy, WolframAlpha </sub>
