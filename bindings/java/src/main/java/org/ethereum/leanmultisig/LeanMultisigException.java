package org.ethereum.leanmultisig;

/** A checked native validation or cryptographic error from leanMultisig. */
public final class LeanMultisigException extends RuntimeException {
    LeanMultisigException(String message) {
        super(message);
    }
}
