#!/usr/bin/env bun
// Release helper used by CI (and runnable locally with bun or node).
//
//   release-manifest.mjs check [tag]
//       Fails unless package.json, Cargo.toml and tauri.conf.json carry the same
//       version (and, when a tag is given, that it is "v<version>"). A mismatch
//       would publish an update the app can never consider installed.
//
//   release-manifest.mjs write <tag> <owner/repo> <bundle-dir> <notes-file> <out>
//       Writes the `latest.json` the in-app updater reads: version, notes and the
//       signed installer for Windows x64.
import { readFileSync, readdirSync, writeFileSync } from "node:fs";
import { dirname, join } from "node:path";
import { fileURLToPath } from "node:url";

const root = join(dirname(fileURLToPath(import.meta.url)), "..");
const fail = (msg) => {
  console.error(`release-manifest: ${msg}`);
  process.exit(1);
};

function versions() {
  const pkg = JSON.parse(readFileSync(join(root, "package.json"), "utf8")).version;
  const conf = JSON.parse(readFileSync(join(root, "src-tauri", "tauri.conf.json"), "utf8")).version;
  const cargo = readFileSync(join(root, "src-tauri", "Cargo.toml"), "utf8").match(/^version\s*=\s*"([^"]+)"/m)?.[1];
  return { "package.json": pkg, "tauri.conf.json": conf, "Cargo.toml": cargo };
}

function check(tag) {
  const v = versions();
  const unique = [...new Set(Object.values(v))];
  if (unique.length !== 1 || !unique[0]) fail(`versions differ: ${JSON.stringify(v)}`);
  if (tag && tag !== `v${unique[0]}`) fail(`tag ${tag} does not match version ${unique[0]} (expected v${unique[0]})`);
  return unique[0];
}

function write(tag, repo, bundleDir, notesFile, out) {
  if (!tag || !repo || !bundleDir || !notesFile || !out) fail("usage: write <tag> <owner/repo> <bundle-dir> <notes-file> <out>");
  const version = check(tag);
  const files = readdirSync(bundleDir);
  const installers = files.filter((f) => f.endsWith("-setup.exe"));
  if (installers.length !== 1) fail(`expected exactly one *-setup.exe in ${bundleDir}, found ${installers.length}`);
  const installer = installers[0];
  if (!installer.includes(version)) fail(`installer ${installer} is not for version ${version}`);
  if (!files.includes(`${installer}.sig`)) {
    fail(`${installer}.sig is missing: the build was not signed (TAURI_SIGNING_PRIVATE_KEY secret not set?)`);
  }
  const signature = readFileSync(join(bundleDir, `${installer}.sig`), "utf8").trim();
  if (!signature) fail("signature file is empty");
  const manifest = {
    version,
    notes: readFileSync(notesFile, "utf8").trim(),
    pub_date: new Date().toISOString(),
    platforms: {
      "windows-x86_64": {
        signature,
        url: `https://github.com/${repo}/releases/download/${tag}/${encodeURIComponent(installer)}`,
      },
    },
  };
  writeFileSync(out, `${JSON.stringify(manifest, null, 2)}\n`);
  console.log(`release-manifest: wrote ${out} for ${installer}`);
}

const [cmd, ...args] = process.argv.slice(2);
if (cmd === "check") console.log(`release-manifest: version ${check(args[0])} is consistent`);
else if (cmd === "write") write(...args);
else fail("usage: release-manifest.mjs check [tag] | write <tag> <owner/repo> <bundle-dir> <notes-file> <out>");
