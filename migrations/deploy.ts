import * as anchor from "@coral-xyz/anchor";

module.exports = async function deploy(_provider: anchor.AnchorProvider) {
  // Deploy is handled via `anchor deploy`.
  // This migration hook exists so future bootstrap logic can be added
  // (e.g. protocol config initialization) without changing scripts.
};
