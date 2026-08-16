const native = require('./index.js')

native.SecretKey.generate = native.generateSecretKey
native.SecretKey.fromSeed = native.secretKeyFromSeed
native.SecretKey.fromBytes = native.secretKeyFromBytes

module.exports = native
