# Poseidon Parameters

Parameters generated using the tools provided by the reference implementation.
You should have `sagemath` installed on your system.

Spec:

* Curve: Bls12-381
* Security: 128bit
* S-box: x^5
* Supported arities: 1 to 16

```sh
git clone https://extgit.iaik.tugraz.at/krypto/hadeshash.git
cd hadeshash/code
for i in {2..17}
do
  sage generate_params_poseidon.sage 1 0 255 $i 5 128 0x73eda753299d7d483339d80809a1d80553bda402fffe5bfeffffffff00000001
done
```
