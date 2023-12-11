# Athena

Intuitive math syntax for a user-friendly [CAS](https://en.wikipedia.org/wiki/Computer_algebra_system)


```
monomial(n) :: (n: N) -> C(R) { sum[k := 0, n] ( x^k ) }

f(x, y) :: (R x R) -> R { sin(x) + y }

F(x, y) :: int ( f(x, y) ) d(x)

J: R^(2 x 2) = jacobi(F)

d_x := J[0, :]
```

<sub> inspired by GeoGebra, Desmos, SymPy, WolframAlpha </sub>
