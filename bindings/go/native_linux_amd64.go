//go:build cgo && linux && amd64

package leanmultisig

// #cgo LDFLAGS: -L${SRCDIR}/internal/native/linux_amd64 -llean_multisig_native -ldl -lpthread -lm
import "C"
