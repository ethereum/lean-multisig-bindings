export declare class Claim {
  constructor(message: Buffer, slot: number)
  get message(): Buffer
  get slot(): number
}

export declare class SecretKey {
  static generate(slotStart: number, slotEnd: number): Promise<SecretKey>
  static fromSeed(seed: Buffer, slotStart: number, slotEnd: number): Promise<SecretKey>
  static fromBytes(data: Buffer): Promise<SecretKey>
  toBytes(): Buffer
  get publicKey(): Buffer
  get slotStart(): number
  get slotEnd(): number
  prepare(slot: number): Promise<void>
  sign(claim: Claim): Promise<Signature>
}

export declare class Signature {
  static fromBytes(data: Buffer, claim: Claim, signers: Buffer[]): Signature
  toBytes(): Buffer
  get claim(): Claim
}

export declare class ClaimSigners {
  constructor(claim: Claim, signers: Buffer[])
  get claim(): Claim
  get signers(): Buffer[]
}

export declare class MultiClaimProof {
  static fromBytes(data: Buffer, groups: ClaimSigners[]): MultiClaimProof
  toBytes(): Buffer
}

export declare function setup(): Promise<void>
export declare function aggregate(signatures: Signature[], claim: Claim): Promise<Signature>
export declare function verify(signature: Signature, expectedSigners: Buffer[], claim: Claim): void
export declare function verifiedSigners(signature: Signature, claim: Claim): Promise<Buffer[]>
export declare function mergeClaims(signatures: Signature[]): Promise<MultiClaimProof>
export declare function verifiedClaims(proof: MultiClaimProof): Promise<ClaimSigners[]>
export declare function verifyClaims(proof: MultiClaimProof, expected: ClaimSigners[]): Promise<void>
