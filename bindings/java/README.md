# leanMultisig Java bindings

Java 25 bindings for [leanMultisig](https://github.com/leanEthereum/leanMultisig). The public API
is a named Java module, `org.ethereum.leanmultisig`, built on Java's finalized Foreign Function &
Memory API (FFM/Panama). It wraps native objects in `AutoCloseable` types; use
try-with-resources for `SecretKey`, `Signature`, and `MultiClaimProof`.

This is an initial source binding, not yet a published Maven artifact. Gradle builds the matching
Rust shared library before running tests.

## Requirements

- JDK 25
- Gradle 9.1 or newer
- Rust (stable toolchain)

## Build and test

From the repository root:

```sh
gradle -p bindings/java test
```

The test task builds the shared `lean_multisig_native` library in release mode and supplies its path through
`-Dlean.multisig.native.path`. Applications embedding the library must similarly build or package
a platform-specific native library and set that property before first use.

## Native-access permission

The library is a named module. A deployed modular application should grant native access narrowly:

```sh
java --enable-native-access=org.ethereum.leanmultisig ...
```

Gradle's JUnit runner executes on the class path, so the repository test task instead uses
`--enable-native-access=ALL-UNNAMED`. This broader permission is test-harness-only; it is not the
recommended flag for a modular Teku integration.

## Example

```java
import java.util.List;
import org.ethereum.leanmultisig.*;

LeanMultisig.setup();
Claim claim = new Claim(message32, 100);
try (SecretKey key = SecretKey.fromSeed(seed32, 100, 115);
     Signature signature = key.sign(claim)) {
    boolean valid = LeanMultisig.verify(signature, List.of(key.publicKey()), claim);
}
```

The native ABI in [`../native`](../native) is an internal implementation detail shared by
managed-language bindings, not a stable C API. Java currently consumes it through FFM; a future
Go binding may consume it through cgo. Signature and proof byte encodings intentionally omit their
claim/signer context; callers provide that context when restoring or verifying them.
