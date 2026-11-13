// Windows restore point before the first risky change of each app run.
import type { Risk } from "./api";
import { backend } from "./mock";

let done: Promise<void> | null = null;

/**
 * Creates a restore point once per run, before a medium/high-risk change.
 * Never blocks the change: a failure is only reported.
 */
export function ensureRestorePoint(risk: Risk, notify: (kind: "ok" | "err", msg: string) => void): Promise<void> {
  if (risk === "low") return Promise.resolve();
  if (!done) {
    notify("ok", "A criar um ponto de restauro do Windows antes da primeira alteração de risco…");
    done = backend
      .restorePointCreate(false)
      .then((r) => notify(r.created ? "ok" : "err", r.message))
      .catch((e) => notify("err", `Ponto de restauro: ${e}`));
  }
  return done;
}
