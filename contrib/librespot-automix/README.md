# The librespot patch behind "Blend songs into each other"

`librespot-automix.patch` adds the transition engine to the librespot
revision that `Cargo.toml` pins: beat and bar analysis, the decoded
opening of the preloaded next song, the overlap with its bass swap, the
filter sweep on the outgoing song, and the drop detection that chooses
where the incoming one enters.

That revision has no crossfade, so the Spotifast code on this branch
needs the patch applied to its librespot checkout to build:

```sh
cd ~/.cargo/git/checkouts/librespot-dadbde7662497620/3ba977f
git apply /path/to/contrib/librespot-automix/librespot-automix.patch
```

Then build Spotifast as usual. A clean checkout without the patch will
fail to compile, because `PlayerConfig::crossfade_ms` and
`Player::set_crossfade` only exist in the patch.

The lasting fix is to send the patch upstream to
librespot-org/librespot and point the dependency at a fork that carries
it once a maintainer picks it up. Until then this file is the only copy
of the engine in the repository.
