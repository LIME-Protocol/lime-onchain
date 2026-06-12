import fs from "fs";
import path from "path";
import { Keypair } from "@solana/web3.js";

const root = path.resolve(path.dirname(new URL(import.meta.url).pathname), "..");
const deployDir = path.join(root, "target", "deploy");

const programs = [
  {
    name: "lime_market",
    keypairFile: "lime_market-keypair.json",
    sourceFile: "programs/lime-market/src/lib.rs",
    anchorTomlName: "lime_market",
  },
  {
    name: "lime_vault",
    keypairFile: "lime_vault-keypair.json",
    sourceFile: "programs/lime-vault/src/lib.rs",
    anchorTomlName: "lime_vault",
  },
  {
    name: "lime_settlement",
    keypairFile: "lime_settlement-keypair.json",
    sourceFile: "programs/lime-settlement/src/lib.rs",
    anchorTomlName: "lime_settlement",
  },
];

fs.mkdirSync(deployDir, { recursive: true });

const existingFiles = programs
  .map((program) => path.join(deployDir, program.keypairFile))
  .filter((file) => fs.existsSync(file));

if (existingFiles.length > 0) {
  const backupDir = path.join(deployDir, `backup-${new Date().toISOString().replace(/[:.]/g, "-")}`);
  fs.mkdirSync(backupDir, { recursive: true });
  for (const file of existingFiles) {
    fs.copyFileSync(file, path.join(backupDir, path.basename(file)));
  }
  console.log(`Backed up existing program keypairs to ${backupDir}`);
}

const ids = new Map();

for (const program of programs) {
  const keypair = Keypair.generate();
  const programId = keypair.publicKey.toBase58();
  ids.set(program.name, programId);

  fs.writeFileSync(
    path.join(deployDir, program.keypairFile),
    JSON.stringify(Array.from(keypair.secretKey)),
  );

  const sourcePath = path.join(root, program.sourceFile);
  const source = fs.readFileSync(sourcePath, "utf8");
  fs.writeFileSync(
    sourcePath,
    source.replace(/declare_id!\("[^"]+"\);/, `declare_id!("${programId}");`),
  );
}

const anchorPath = path.join(root, "Anchor.toml");
let anchorToml = fs.readFileSync(anchorPath, "utf8");
for (const program of programs) {
  anchorToml = anchorToml.replace(
    new RegExp(`(${program.anchorTomlName}\\s*=\\s*")[^"]+(")`),
    `$1${ids.get(program.name)}$2`,
  );
}
fs.writeFileSync(anchorPath, anchorToml);

console.log("Generated fresh program IDs:");
for (const program of programs) {
  console.log(`${program.name}=${ids.get(program.name)}`);
}
