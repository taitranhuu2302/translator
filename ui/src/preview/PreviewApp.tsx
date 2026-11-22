/**
 * The design preview harness.
 *
 * Every surface the app can show, drawn from the real components with made-up
 * data, so the layout and the theme can be judged before the app is launched.
 * It is dev only: `preview.html` is not an input of the production build.
 *
 *   npm run dev   →   http://localhost:1420/preview.html
 *
 * Nothing here is imported by the app, and nothing here reaches across the IPC
 * seam: if a scene needs a value, it is written out below by hand.
 */
import { useEffect, useState } from "react";

import { Select, SelectContent, SelectItem, SelectTrigger, SelectValue } from "@/components/ui/select";
import { Separator } from "@/components/ui/separator";
import { AppShell } from "@/components/AppShell";
import { HistorySheet } from "@/components/HistorySheet";
import { SettingsSheet } from "@/components/SettingsSheet";
import { TranslatePanes } from "@/components/TranslatePanes";

import type { AnswerToken } from "../answerToken";
import { ReplaceHudCard } from "../HudView";
import { EMPTY_IMPROVEMENT, type ImproveResult } from "../improveWindow";
import type {
  HistoryEntry,
  ImprovePart,
  Mode,
  ReplacePhase,
  Translation,
} from "../ipc";
import { MOCK_SETTINGS } from "./mockIpc";
import { CaptureNoticeView, NoticeView } from "../NoticeView";
import { Onboarding } from "../Onboarding";
import type { Notice } from "../notice";
import { EMPTY_RESULT, type TranslateResult } from "../translateWindow";

const TOKEN: AnswerToken = { session: 1, sequence: 1 };

const SOURCE_EN =
  "Select text in any app and press the shortcut. The window opens with that text already in the Source pane, and the translation streams into the pane beside it.";

const SOURCE_EN_LONG = `${SOURCE_EN}

Long text is the interesting case: the result pane scrolls on its own, the source keeps its scroll position, and the footer never leaves the bottom of the window. ${SOURCE_EN}`;

const TRANSLATION_VI =
  "Chọn văn bản trong bất kỳ ứng dụng nào rồi nhấn phím tắt. Cửa sổ mở ra với đoạn văn bản đó đã nằm trong khung Nguồn, và bản dịch hiện dần ở khung bên cạnh.";

const TRANSLATION_VI_LONG = `${TRANSLATION_VI}

Văn bản dài là trường hợp đáng chú ý nhất: khung kết quả tự cuộn, khung nguồn giữ nguyên vị trí cuộn, và thanh dưới cùng không bao giờ rời khỏi đáy cửa sổ. ${TRANSLATION_VI}`;

const TRANSLATION: Translation = {
  text: TRANSLATION_VI,
  provider: "azure",
  source: "en",
  detected_source: "en",
  target: "vi",
};

const TRANSLATION_LONG: Translation = {
  text: TRANSLATION_VI_LONG,
  provider: "my_memory",
  source: "en",
  detected_source: "en",
  target: "vi",
};

const CORRECTED =
  "Select text in any app and press the shortcut. The window opens with that text already in the Source pane, and the translation streams into the pane beside it.";

const BETTER =
  "Press the shortcut with any text selected, and the window opens with it already in the Source pane — the translation writes itself into the pane beside it.";

const NOTICES: Notice[] = [
  {
    kind: "capture",
    title: "Neris needs Accessibility to read your selection.",
    detail:
      "Open System Settings → Privacy & Security → Accessibility, enable Neris, then press the shortcut again.",
    action: {
      id: "open_accessibility_settings",
      label: "Open System Settings",
    },
  },
  {
    kind: "capture",
    title: "This is a secure field, so nothing was read.",
    detail:
      "Passwords stay on your Mac. Finish there, then try again elsewhere.",
  },
  {
    kind: "offline",
    title: "You appear to be offline.",
    detail: "Neris needs a network connection. Reconnect, then try again.",
  },
  {
    kind: "provider",
    title: "Groq needs an API key.",
    detail: "Add a Groq key under Provider keys in Settings, then try again.",
  },
  {
    kind: "error",
    title: "The translation did not finish.",
    detail: "Try again.",
  },
];

type State = "empty" | "busy" | "done" | "failure";

type Overlay =
  | null
  | { kind: "history"; entries: HistoryEntry[]; error: string | null }
  | { kind: "settings"; error: string | null }
  | { kind: "onboarding" };

interface Scene {
  id: string;
  mode: Mode;
  state: State;
  label: string;
  overlay?: Overlay;
}

const NOW = Math.floor(Date.now() / 1000);

const HISTORY_ENTRIES: HistoryEntry[] = [
  {
    source: SOURCE_EN,
    result: TRANSLATION_VI,
    provider: "azure",
    mode: "translate",
    created_at: NOW - 90,
    direction: { source: "en", target: "vi" },
    corrected: null,
  },
  {
    source: "this sentence have a mistake in it.",
    result: BETTER,
    provider: "groq",
    mode: "improve",
    created_at: NOW - 60 * 42,
    direction: null,
    corrected: CORRECTED,
  },
  {
    source: "Lưu bản dịch vào lịch sử để mở lại sau.",
    result: "Saves the translation to History so it can be opened again later.",
    provider: "my_memory",
    mode: "translate",
    created_at: NOW - 60 * 60 * 30,
    direction: { source: "vi", target: "en" },
    corrected: null,
  },
];

const SCENES: Scene[] = [
  { id: "translate-empty", mode: "translate", state: "empty", label: "Translate · empty" },
  { id: "translate-busy", mode: "translate", state: "busy", label: "Translate · translating" },
  { id: "translate-done", mode: "translate", state: "done", label: "Translate · translated" },
  { id: "translate-long", mode: "translate", state: "done", label: "Translate · long text" },
  { id: "translate-failure", mode: "translate", state: "failure", label: "Translate · no Accessibility" },
  { id: "translate-offline", mode: "translate", state: "failure", label: "Translate · offline" },
  { id: "improve-empty", mode: "improve", state: "empty", label: "Improve · empty" },
  { id: "improve-busy", mode: "improve", state: "busy", label: "Improve · streaming" },
  { id: "improve-done", mode: "improve", state: "done", label: "Improve · improved" },
  { id: "improve-failure", mode: "improve", state: "failure", label: "Improve · missing key" },
  { id: "history-empty", mode: "translate", state: "done", label: "History · empty", overlay: { kind: "history", entries: [], error: null } },
  { id: "history-list", mode: "translate", state: "done", label: "History · entries", overlay: { kind: "history", entries: HISTORY_ENTRIES, error: null } },
  { id: "history-failure", mode: "translate", state: "done", label: "History · could not be read", overlay: { kind: "history", entries: [], error: "History could not be read." } },
  { id: "settings", mode: "translate", state: "done", label: "Settings · sheet", overlay: { kind: "settings", error: null } },
  {
    id: "settings-failure",
    mode: "translate",
    state: "done",
    label: "Settings · could not be read",
    overlay: {
      kind: "settings",
      error:
        "The settings file could not be read: expected a string at line 4, column 15.",
    },
  },
  { id: "notices", mode: "translate", state: "failure", label: "Notices · all kinds" },
  { id: "hud", mode: "translate", state: "empty", label: "Replace · progress HUD" },
  { id: "onboarding", mode: "translate", state: "done", label: "Onboarding · welcome", overlay: { kind: "onboarding" } },
];

function resultFor(scene: Scene): TranslateResult {
  switch (scene.id) {
    case "translate-busy":
      return { phase: "translating", token: TOKEN };
    case "translate-done":
      return { phase: "translated", translation: TRANSLATION };
    case "translate-long":
      return { phase: "translated", translation: TRANSLATION_LONG };
    case "translate-failure":
      return { phase: "failed", ...NOTICES[0] };
    case "translate-offline":
      return { phase: "failed", ...NOTICES[2] };
    case "notices":
      return { phase: "failed", ...NOTICES[3] };
    default:
      return EMPTY_RESULT;
  }
}

function improvementFor(scene: Scene): ImproveResult {
  switch (scene.id) {
    case "improve-busy":
      return {
        phase: "improving",
        token: TOKEN,
        corrected: CORRECTED,
        better: "",
      };
    case "improve-done":
      return { phase: "improved", corrected: CORRECTED, better: BETTER };
    case "improve-failure":
      return { phase: "failed", ...NOTICES[3] };
    default:
      return EMPTY_IMPROVEMENT;
  }
}

function sourceFor(scene: Scene): string {
  switch (scene.id) {
    case "translate-empty":
    case "improve-empty":
      return "";
    case "translate-long":
      return SOURCE_EN_LONG;
    case "translate-done":
    case "translate-busy":
      return SOURCE_EN;
    default:
      return SOURCE_EN;
  }
}

/** The scene's id is kept in the URL hash, so any state can be linked to and
 * reloaded without hunting through the picker. */
function sceneFromHash(): string {
  const id = decodeURIComponent(window.location.hash.replace(/^#/, ""));
  return SCENES.some((scene) => scene.id === id) ? id : "translate-done";
}

export function PreviewApp() {
  const [sceneId, setScene] = useState(sceneFromHash);
  const setSceneId = (id: string) => {
    window.location.hash = id;
    setScene(id);
  };
  const scene = SCENES.find((candidate) => candidate.id === sceneId) ?? SCENES[0];
  const [historyQuery, setHistoryQuery] = useState("");

  useEffect(() => {
    const onHashChange = () => setScene(sceneFromHash());
    window.addEventListener("hashchange", onHashChange);
    return () => window.removeEventListener("hashchange", onHashChange);
  }, []);

  // The mode switch follows the scene family: flipping it in the toolbar shows
  // the same state on the other surface.
  const switchMode = (mode: Mode) => {
    const next =
      SCENES.find((candidate) => candidate.mode === mode && candidate.state === scene.state) ??
      SCENES.find((candidate) => candidate.mode === mode && candidate.state === "empty");
    if (next) setSceneId(next.id);
  };

  return (
    <div className="flex h-full flex-col">
      <PreviewBar scene={scene} onScene={setSceneId} />

      <div className="min-h-0 flex-1">
        <AppShell
          mode={scene.mode}
          onModeChange={switchMode}
          onOpenHistory={() => undefined}
          onOpenSettings={() => undefined}
          translateShortcut="⌃⌥E"
          replaceShortcut="⌃⌥R"
        >
          {scene.id === "notices" ? (
            <NoticesScene />
          ) : scene.id === "hud" ? (
            <HudScene />
          ) : (
            <TranslatePanes
              mode={scene.mode}
              text={sourceFor(scene)}
              onTextChange={() => undefined}
              onSubmit={() => undefined}
              result={resultFor(scene)}
              improvement={improvementFor(scene)}
              copied={false}
              copiedPart={null}
              onCopy={() => undefined}
              onCopyPart={(_part: ImprovePart) => undefined}
              onSwap={() => undefined}
              canSwap={scene.state === "done"}
              shortcut="⌃⌥E"
            />
          )}

          {scene.overlay?.kind === "history" ? (
            <HistorySheet
              open
              onOpenChange={(open) => {
                if (!open) setSceneId("translate-done");
              }}
              entries={scene.overlay.entries}
              query={historyQuery}
              error={scene.overlay.error}
              onQueryChange={setHistoryQuery}
              onOpen={() => setSceneId("translate-done")}
            />
          ) : null}

          {scene.overlay?.kind === "settings" ? (
            <SettingsSheet
              open
              onOpenChange={(open) => {
                if (!open) setSceneId("translate-done");
              }}
              settings={scene.overlay.error ? null : MOCK_SETTINGS}
              error={scene.overlay.error}
              onSettingsSaved={() => undefined}
            />
          ) : null}

          {scene.overlay?.kind === "onboarding" ? (
            <Onboarding
              key={scene.id}
              shortcuts={MOCK_SETTINGS.shortcuts}
              granted={false}
              onDone={async () => setSceneId("translate-done")}
            />
          ) : null}
        </AppShell>
      </div>
    </div>
  );
}

function PreviewBar({
  scene,
  onScene,
}: {
  scene: Scene;
  onScene: (id: string) => void;
}) {
  return (
    <div className="flex h-10 shrink-0 items-center gap-3 border-b border-border bg-popover px-3 text-xs text-muted-foreground">
      <span className="font-medium text-foreground">Design preview</span>
      <Separator orientation="vertical" className="h-4" />
      <Select value={scene.id} onValueChange={(value) => onScene(value as string)}>
        <SelectTrigger size="sm" className="w-56">
          <SelectValue />
        </SelectTrigger>
        <SelectContent>
          {SCENES.map((candidate) => (
            <SelectItem key={candidate.id} value={candidate.id}>
              {candidate.label}
            </SelectItem>
          ))}
        </SelectContent>
      </Select>
      <span className="ml-auto">Dev only — never shipped</span>
    </div>
  );
}

/**
 * The Replace HUD at each of its steps, in the 260 × 52 window the shell gives
 * it. The dashed frame stands in for the transparent window; the capsule is the
 * component the page itself draws.
 */
function HudScene() {
  const phases: ReplacePhase[] = ["capturing", "translating", "replacing"];
  return (
    <div className="min-h-0 flex-1 overflow-y-auto p-6">
      <div className="flex flex-col gap-5">
        {phases.map((phase) => (
          <div key={phase} className="flex flex-col gap-2">
            <span className="text-xs text-muted-foreground">{phase}</span>
            <div className="flex h-[52px] w-[260px] items-center rounded-lg p-2.5 outline outline-dashed outline-border">
              <ReplaceHudCard phase={phase} />
            </div>
          </div>
        ))}
      </div>
    </div>
  );
}

/** Every notice kind in one column, so their tints can be compared side by side,
 * and the Accessibility one a second time through the resolving wrapper: the
 * granted card is the model's own wording, not a sentence copied into the
 * harness. */
function NoticesScene() {
  return (
    <div className="min-h-0 flex-1 overflow-y-auto p-4">
      <div className="mx-auto flex max-w-xl flex-col gap-3">
        {NOTICES.map((notice) => (
          <NoticeView key={notice.title} notice={notice} />
        ))}
        <CaptureNoticeView granted shortcut="⌃⌥E" notice={NOTICES[0]} />
      </div>
    </div>
  );
}
