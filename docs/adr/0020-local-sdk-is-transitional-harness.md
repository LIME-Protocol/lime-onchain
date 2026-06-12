# Local SDK Is Transitional Harness

Any SDK-like code that remains in `lime-onchain` is a transitional harness for tests, bootstrapping, and extraction work, not the public SDK surface. New public integration APIs should be designed for `LIME-Protocol/lime-sdk`, while this repository should avoid growing a second SDK except where needed to keep Program tests and deployment scripts working during the transition.
