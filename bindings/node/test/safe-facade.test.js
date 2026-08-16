const assert = require('node:assert/strict')
const test = require('node:test')

const multisig = require('..')

test('signs, aggregates, and verifies a seeded key', async () => {
  await multisig.setup()

  const seed = Buffer.alloc(32, 7)
  const message = Buffer.alloc(32, 9)
  const claim = new multisig.Claim(message, 100)
  const key = await multisig.SecretKey.fromSeed(seed, 100, 115)
  const signature = await key.sign(claim)
  const aggregate = await multisig.aggregate([signature], claim)

  assert.deepEqual(aggregate.claim.message, message)
  assert.equal(aggregate.claim.slot, 100)
  assert.doesNotThrow(() => multisig.verify(aggregate, [key.publicKey], claim))
})
