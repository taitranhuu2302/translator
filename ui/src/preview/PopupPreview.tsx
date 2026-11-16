/**
 * The Quick Translate card, reviewed in a browser.
 *
 * The real popup is a transparent 420-wide window whose height the shell sets
 * from what this page measured. A browser has no shell, so the frame here
 * stands in for the window and applies the same two bounds the Rust clamp does
 * — that is the only thing the preview decides on its own.
 *
 * Dev only: `preview-popup.html` is not an input of the production build.
 */
import { StrictMode, useSyncExternalStore } from "react";
import { createRoot } from "react-dom/client";

import { Button } from "@/components/ui/button";

import { ipc, type PopupPrefill, type Translation } from "../ipc";
import { Popup } from "../PopupView";
import "../index.css";

/** Mirrors `pointer::MIN_POPUP_HEIGHT` / `MAX_POPUP_HEIGHT`. The shell owns the
 * real clamp; the frame needs a number to draw with. */
const MIN_HEIGHT = 120;
const MAX_HEIGHT = 320;
const WIDTH = 420;

const SHORT_SOURCE = "Chính sách bảo mật của chúng tôi vừa được cập nhật.";

const LONG_SOURCE =
  "Chúng tôi đã cập nhật chính sách bảo mật để giải thích rõ hơn dữ liệu nào được thu thập, dữ liệu đó được giữ trong bao lâu, và những lựa chọn bạn có.";

const SHORT_ANSWER: Translation = {
  text: "Our privacy policy has just been updated.",
  provider: "azure",
  detected_source: "vi",
  target: "en",
};

const LONG_ANSWER: Translation = {
  text: "We have updated our privacy policy to explain more clearly what data is collected, how long it is kept, and which choices you have. The new policy takes effect on 1 December and replaces the version published last spring. Nothing about the way Translator itself works has changed: the text you select is still sent only to the provider you have configured, and nothing is stored on this Mac beyond your history. If you have questions, you can reply to the email that announced the change, and a person will answer.",
  provider: "azure",
  detected_source: "vi",
  target: "en",
};

type SceneId = "translated" | "long" | "failure" | "translating";

const SCENES: { id: SceneId; label: string }[] = [
  { id: "translated", label: "Translated" },
  { id: "long", label: "Long answer" },
  { id: "failure", label: "No key" },
  { id: "translating", label: "Translating" },
];

/**
 * The harness state the mock reads. It lives outside React so the card's own
 * effects — which run before its parent's — always see the scene they were
 * mounted for.
 */
const harness = {
  scene: "translated" as SceneId,
  reported: 0,
  listeners: new Set<() => void>(),
};

function reportHeight(height: number) {
  harness.reported = height;
  for (const listener of harness.listeners) listener();
}

function subscribe(listener: () => void) {
  harness.listeners.add(listener);
  return () => {
    harness.listeners.delete(listener);
  };
}

/** The answers this page needs, and nothing else: the harness never pretends to
 * be the engine, it only spares the card an IPC failure. */
function installMock() {
  Object.assign(ipc, {
    takePopupPrefill: async (): Promise<PopupPrefill> => ({
      selection: {
        text: harness.scene === "long" ? LONG_SOURCE : SHORT_SOURCE,
      },
    }),
    translateText: async (): Promise<{ translation: Translation }> => {
      // Every scene starts as "translating"; the answer or the failure then
      // replaces it. The scene that is *about* translating holds there.
      if (harness.scene === "translating") {
        await new Promise(() => {
          // Never settles: the card stays in the state being reviewed.
        });
      }
      await new Promise((resolve) => setTimeout(resolve, 900));
      if (harness.scene === "failure") {
        throw { provider: { missing_key: { provider: "groq" } } };
      }
      return {
        translation: harness.scene === "long" ? LONG_ANSWER : SHORT_ANSWER,
      };
    },
    replaceWithTranslation: async () => {
      throw new Error("Replace is not part of the review");
    },
    hidePopup: async () => undefined,
    copyText: async () => undefined,
    setPopupHeight: async (height: number) => {
      reportHeight(height);
    },
  });
}

function openScene(scene: SceneId) {
  harness.scene = scene;
  reportHeight(0);
}

export function PopupPreview() {
  const measured = useSyncExternalStore(subscribe, () => harness.reported);
  const height = Math.min(Math.max(measured, MIN_HEIGHT), MAX_HEIGHT);

  return (
    <div className="flex min-h-full flex-col bg-background p-6 text-foreground">
      <div className="flex flex-wrap items-center gap-2 pb-4">
        {SCENES.map((entry) => (
          <Button
            key={entry.id}
            size="xs"
            variant={entry.id === harness.scene ? "secondary" : "ghost"}
            onClick={() => openScene(entry.id)}
          >
            {entry.label}
          </Button>
        ))}
        <span className="ml-auto font-mono text-[0.6875rem] text-muted-foreground">
          window {WIDTH} × {height} · card reports {measured || "—"}
        </span>
      </div>

      <div className="mx-auto flex flex-col gap-2">
        {/* The popup window, standing in for the transparent one the shell
            sizes. Its height is the reported content height, clamped. */}
        <div
          style={{ width: WIDTH, height }}
          className="rounded-xl bg-[repeating-linear-gradient(45deg,transparent,transparent_6px,rgba(255,255,255,0.05)_6px,rgba(255,255,255,0.05)_12px)]"
        >
          <Popup key={harness.scene} />
        </div>
        <p className="text-center text-[0.6875rem] text-muted-foreground">
          The hatched area is the transparent window around the card. Dev only —
          never shipped.
        </p>
      </div>
    </div>
  );
}

installMock();

const container = document.getElementById("root");
if (!container) {
  throw new Error("preview-popup.html is missing #root");
}
createRoot(container).render(
  <StrictMode>
    <PopupPreview />
  </StrictMode>,
);
