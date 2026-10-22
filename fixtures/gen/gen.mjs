// Generates ../../tests/fixtures/vectors.json from independent JS implementations
// (@solana/web3.js, @solana/spl-token, mpl-token-metadata via umi, tweetnacl).
// Deterministic: fixed seeds, fixed blockhash, ed25519 signatures are deterministic.
import { writeFileSync, mkdirSync } from "node:fs";
import { dirname, resolve } from "node:path";
import { fileURLToPath } from "node:url";
import nacl from "tweetnacl";
import bs58 from "bs58";
import {
  Keypair,
  PublicKey,
  SystemProgram,
  TransactionMessage,
} from "@solana/web3.js";
import {
  TOKEN_PROGRAM_ID,
  ASSOCIATED_TOKEN_PROGRAM_ID,
  getAssociatedTokenAddressSync,
  createAssociatedTokenAccountIdempotentInstruction,
  createMintToCheckedInstruction,
  createInitializeMint2Instruction,
} from "@solana/spl-token";
import { createUmi } from "@metaplex-foundation/umi-bundle-defaults";
import {
  createNoopSigner,
  publicKey as umiKey,
  none,
  some,
} from "@metaplex-foundation/umi";
import {
  mplTokenMetadata,
  createMetadataAccountV3,
  updateMetadataAccountV2,
  MPL_TOKEN_METADATA_PROGRAM_ID,
} from "@metaplex-foundation/mpl-token-metadata";

const NAME = "CrackCoin";
const SYMBOL = "CRACK";
const URI =
  "https://raw.githubusercontent.com/cycle-five/crack-coin/master/assets/crackcoin.json";
const MINT_LAMPORTS = 1461600;
const MINT_LEN = 82;
const AMOUNT = 500;

const hex = (b) => Buffer.from(b).toString("hex");
const shortvec = (n) => {
  const out = [];
  let v = n;
  for (;;) {
    const byte = v & 0x7f;
    v >>= 7;
    if (v === 0) {
      out.push(byte);
      return Buffer.from(out);
    }
    out.push(byte | 0x80);
  }
};
const b58 = (b) => new PublicKey(b).toBase58();

const METADATA_PROGRAM = new PublicKey(MPL_TOKEN_METADATA_PROGRAM_ID);
const programs = {
  system: SystemProgram.programId.toBase58(),
  token: TOKEN_PROGRAM_ID.toBase58(),
  associated_token: ASSOCIATED_TOKEN_PROGRAM_ID.toBase58(),
  metadata: METADATA_PROGRAM.toBase58(),
};

const seedKey = (n) => {
  const seed = new Uint8Array(32).fill(n);
  const kp = Keypair.fromSeed(seed);
  return { kp, entry: { seed_hex: hex(seed), pubkey: kp.publicKey.toBase58() } };
};
const a = seedKey(1);
const o = seedKey(2);
const m = seedKey(3);
const authority = a.kp;
const owner = o.kp.publicKey;
const mint = m.kp.publicKey;

const blockhash = b58(new Uint8Array(32).fill(4));

const ataAddress = getAssociatedTokenAddressSync(mint, owner);
const [ataPda, ataBump] = PublicKey.findProgramAddressSync(
  [owner.toBuffer(), TOKEN_PROGRAM_ID.toBuffer(), mint.toBuffer()],
  ASSOCIATED_TOKEN_PROGRAM_ID,
);
if (!ataPda.equals(ataAddress)) throw new Error("ATA derivation mismatch");

const [metadataAddress, metadataBump] = PublicKey.findProgramAddressSync(
  [Buffer.from("metadata"), METADATA_PROGRAM.toBuffer(), mint.toBuffer()],
  METADATA_PROGRAM,
);

const onCurve = [
  authority.publicKey,
  owner,
  mint,
  ataAddress,
  metadataAddress,
].map((k) => ({ pubkey: k.toBase58(), on_curve: PublicKey.isOnCurve(k.toBytes()) }));

// Compile, sign with each signer in the message's signer order, wire it.
const buildTx = (instructions, signerKps) => {
  const message = new TransactionMessage({
    payerKey: authority.publicKey,
    recentBlockhash: blockhash,
    instructions,
  }).compileToLegacyMessage();
  const bytes = Buffer.from(message.serialize());
  const required = message.header.numRequiredSignatures;
  const order = message.staticAccountKeys.slice(0, required);
  const signatures = order.map((k) => {
    const kp = signerKps.find((s) => s.publicKey.equals(k));
    if (!kp) throw new Error(`no signer for ${k.toBase58()}`);
    return Buffer.from(nacl.sign.detached(bytes, kp.secretKey));
  });
  const wire = Buffer.concat([shortvec(signatures.length), ...signatures, bytes]);
  return { bytes, signatures, wire };
};

const mintTx = buildTx(
  [
    createAssociatedTokenAccountIdempotentInstruction(
      authority.publicKey,
      ataAddress,
      owner,
      mint,
    ),
    createMintToCheckedInstruction(mint, ataAddress, authority.publicKey, AMOUNT, 0),
  ],
  [authority],
);
if (mintTx.signatures.length !== 1) throw new Error("mint_tx expects one signer");

const createMintTx = buildTx(
  [
    SystemProgram.createAccount({
      fromPubkey: authority.publicKey,
      newAccountPubkey: mint,
      lamports: MINT_LAMPORTS,
      space: MINT_LEN,
      programId: TOKEN_PROGRAM_ID,
    }),
    createInitializeMint2Instruction(mint, 0, authority.publicKey, null),
  ],
  [authority, m.kp],
);

// Metadata instructions through umi; building them needs no network.
const umi = createUmi("http://127.0.0.1:8899").use(mplTokenMetadata());
const auth = createNoopSigner(umiKey(authority.publicKey.toBase58()));
const metadataKey = umiKey(metadataAddress.toBase58());
const dumpIx = (builder) => {
  const ixs = builder.getInstructions();
  if (ixs.length !== 1) throw new Error("expected one instruction");
  const ix = ixs[0];
  return {
    program: ix.programId.toString(),
    accounts: ix.keys.map((k) => ({
      pubkey: k.pubkey.toString(),
      is_signer: k.isSigner,
      is_writable: k.isWritable,
    })),
    data_hex: hex(ix.data),
  };
};
const tokenData = {
  name: NAME,
  symbol: SYMBOL,
  uri: URI,
  sellerFeeBasisPoints: 0,
  creators: none(),
  collection: none(),
  uses: none(),
};
const metadataCreateIx = dumpIx(
  createMetadataAccountV3(umi, {
    metadata: metadataKey,
    mint: umiKey(mint.toBase58()),
    mintAuthority: auth,
    payer: auth,
    updateAuthority: auth,
    data: tokenData,
    isMutable: true,
    collectionDetails: none(),
  }),
);
const metadataUpdateIx = dumpIx(
  updateMetadataAccountV2(umi, {
    metadata: metadataKey,
    updateAuthority: auth,
    data: some(tokenData),
    newUpdateAuthority: none(),
    primarySaleHappened: none(),
    isMutable: none(),
  }),
);

const vectors = {
  programs,
  keys: { authority: a.entry, owner: o.entry, mint: m.entry },
  blockhash,
  on_curve: onCurve,
  ata: {
    owner: owner.toBase58(),
    mint: mint.toBase58(),
    address: ataAddress.toBase58(),
    bump: ataBump,
  },
  metadata_address: {
    mint: mint.toBase58(),
    address: metadataAddress.toBase58(),
    bump: metadataBump,
  },
  mint_tx: {
    amount: AMOUNT,
    message_hex: hex(mintTx.bytes),
    signature: bs58.encode(mintTx.signatures[0]),
    wire_hex: hex(mintTx.wire),
  },
  create_mint_tx: {
    lamports: MINT_LAMPORTS,
    message_hex: hex(createMintTx.bytes),
    signatures: createMintTx.signatures.map((s) => bs58.encode(s)),
    wire_hex: hex(createMintTx.wire),
  },
  metadata_create_ix: metadataCreateIx,
  metadata_update_ix: metadataUpdateIx,
};

const out = resolve(dirname(fileURLToPath(import.meta.url)), "../../tests/fixtures/vectors.json");
mkdirSync(dirname(out), { recursive: true });
writeFileSync(out, JSON.stringify(vectors, null, 2) + "\n");
console.log(`wrote ${out}`);
