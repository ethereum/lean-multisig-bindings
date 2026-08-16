//go:build cgo && linux && arm64

package leanmultisig

// #cgo LDFLAGS: -L${SRCDIR}/internal/native/linux_arm64 -llean_multisig_native -ldl -lpthread -lm
import "C"
