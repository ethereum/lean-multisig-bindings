# leanMultisig Node.js bindings

Node.js bindings for leanMultisig, implemented with Rust and `napi-rs`. The public API wraps the
safe `lean_multisig_api` facade directly; it does not expose the repository's internal C bridge.

Key generation, preparation, signing, setup, and aggregation return Promises so their native work
does not block the JavaScript event loop.

```js
const multisig = require('@ethereum/lean-multisig')

const seed = Buffer.alloc(32)
const message = Buffer.alloc(32)
const claim = new multisig.Claim(message, 100)

await multisig.setup()
const key = await multisig.SecretKey.fromSeed(seed, 100, 115)
const signature = await key.sign(claim)
multisig.verify(signature, [key.publicKey], claim)
```

For development, install Node.js 18.17 or later and Rust, then run:

```sh
cd bindings/node
npm install
npm run build
npm test
```
