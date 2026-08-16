//go:build cgo && darwin && (amd64 || arm64)

package leanmultisig

// #cgo LDFLAGS: -L${SRCDIR}/internal/native/darwin_universal -llean_multisig_native -lm
import "C"
