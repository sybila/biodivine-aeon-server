# Foreword

Welcome! 

This is the manual of AEON, a tool for *long term analysis of [Boolean networks](https://en.wikipedia.org/wiki/Boolean_network)*. In particular, AEON allows you to define Boolean networks with *partially unknown update functions*. Then, for such a network, you can compute its *asynchronous attractors* and explore the *state space* of these attractors. Finally, you can investigate how the attractor structure changes depending on the unknown parts of the network (*attractor bifurcation*) as well as which variables are *stable*, *unstable*, or *switched* under different conditions (*stability analysis*). For all of this, AEON uses efficient symbolic algorithms based on [BDDs](https://en.wikipedia.org/wiki/Binary_decision_diagram), which make it possible to handle even networks with 1000 or more variables. 

In this document, you can find information on how to use AEON to:
 - Create a Boolean network or import it from an existing `.sbml` or `.aeon` file.
 - Create a partially unknown Boolean network with logical parameters.
 - Check that the network is consistent with its regulatory graph.  
 - Compute asynchronous attractors of such a Boolean network.
 - Visualize the state space of the computed attractors.
 - Construct a *bifurcation decision tree*: A visual representation of the dependence between network parameters and attractors.
 - Perform stability analysis of the attractors in different conditions.

AEON is a constantly evolving academic project. If you have any problem or run into some unexpected behaviour, please contact us at `sybila@fi.muni.cz`. We will be happy to help you and make AEON a more useful tool for you. Finally, if you found AEON useful in your research, please cite it using the following publication:

```
Beneš, N., Brim, L., Kadlecaj, J., Pastva, S., & Šafránek, D. (2020, July). 
AEON: attractor bifurcation analysis of parametrised boolean networks. 
In International Conference on Computer Aided Verification (pp. 569-581).
```