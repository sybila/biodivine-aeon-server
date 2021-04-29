# Parametrised networks

In practice, we often don't know the exact update functions that govern the behaviour of the network. In AEON, you can therefore analyse even a network that is *partially unknown*. In such case, we say that the network has *logical parameters*. These parameters can be of two types:
 
 - We say that a variable is *implicitly parametrised* when its entire update function is unknown. For example, in the model from the previous section, since we did not set any update function for `A` or `B`, they are both implicitly parametrised.
 - An update function is *explicitly parametrised* when it contains unknown Boolean expressions. These are expressed using uninterpreted functions. For example, in the model from the previous section, we could set the update function of `C` to be `C | f(A, B)`. Here, `f` is an explicit parameter: an uninterpreted function of arity 2. This way, we are giving a partial specification of the update function: we know that when `C=true`, the value of the function is also `true`. However, if `C=false`, we know the result depends on `A` and `B`, but we don't know what that dependence looks like. 

> An uninterpreted function of arity zero can be also used without argument parenthesis, in which case it resembles a more traditional logical parameter. For example, we can write `(K => A) & (!K => B)` where `A` and `B` are variables, but `K` is an explicit parameter.

> At the moment AEON only allows variables as arguments of uninterpreted functions. That is, you cannot use complex expressions such as `C | f(A & B, C)`. This restriction may be lifted in the future. 

In AEON, we then consider *all possible instantiations* of these logical parameters. That is, AEON considers every possible Boolean function that can be substituted in place of either the explicit or implicit uninterpreted functions. 

> Note that the number of such functions grows very quickly (there are `2^(2^n)` Boolean functions with `n` arguments). For this reason, AEON typically cannot handle uninterpreted functions (explicit or implicit) with more than 5 arguments (which corresponds to `2^32` possible functions).   

In the user interface, you can see this in the form of `Possible instantiations: ...` label under each update function input field. If the function is fully specified (no parameters), there should be one instantiation: the function itself. As soon as parameters are introduced (or the function is removed completely), the number of instantiations should grow.

For example, update functions for variables `A` and `B` from our example should have two instantiations (`true` and `false`), while the `C | f(A, B)` function will have 10 instantiations, as there are 10 binary Boolean functions that depend on both inputs (There are 16 binary Boolean functions, but 6 of them, `true`, `false`, `A`, `!A`, `B`, and `!B` do not depend on both inputs). 

Parameters will come into play later, when we discuss the long term behaviour of Boolean networks. Now, lets look at how we can reduce the number of possible instantiations of a parametrised network by enhancing the regulatory graph with additional structural properties.   