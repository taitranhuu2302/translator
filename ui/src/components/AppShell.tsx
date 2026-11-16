import type { ReactNode } from "react";
import { HistoryIcon, SettingsIcon } from "lucide-react";

import { Button } from "@/components/ui/button";
import { Kbd } from "@/components/ui/kbd";
import { Tabs, TabsList, TabsTrigger } from "@/components/ui/tabs";
import {
  Tooltip,
  TooltipContent,
  TooltipProvider,
  TooltipTrigger,
} from "@/components/ui/tooltip";
import { cn } from "@/lib/utils";

import type { Mode } from "../ipc";

/** macOS draws the traffic lights in the top-leading corner of an overlaid title
 * bar; the toolbar pads past them so nothing sits underneath. */
const TRAFFIC_LIGHT_INSET = "pl-[78px]";

/**
 * The window frame: a unified toolbar the content extends into, the mode switch,
 * the trailing panel buttons, and a footer naming the two keys. The provider is
 * not here: it is only known once an answer arrives, so it is a badge on that
 * answer (see `TranslatePanes`).
 *
 * It holds no state and calls no IPC — every surface inside it is handed its
 * data, which is also how the preview harness draws the real app.
 */
export function AppShell({
  mode,
  onModeChange,
  onOpenHistory,
  onOpenSettings,
  translateShortcut,
  replaceShortcut,
  children,
}: {
  mode: Mode;
  onModeChange: (mode: Mode) => void;
  onOpenHistory: () => void;
  onOpenSettings: () => void;
  translateShortcut: string;
  replaceShortcut: string;
  children: ReactNode;
}) {
  return (
    <TooltipProvider delay={400}>
      <div className="flex h-full min-h-0 flex-col overflow-hidden bg-background text-foreground">
        {/* `data-tauri-drag-region` is what the shell listens for; it applies to
            the element that carries it, so every control below keeps its own
            clicks. */}
        <header
          data-tauri-drag-region
          className={cn(
            "flex h-12 shrink-0 items-center gap-3 border-b border-border/60 pr-3",
            TRAFFIC_LIGHT_INSET,
          )}
        >
          <Tabs
            value={mode}
            onValueChange={(value) => onModeChange(value as Mode)}
            className="gap-0"
          >
            <TabsList className="h-8 gap-0.5 rounded-lg bg-muted/80 p-0.5">
              <TabsTrigger value="translate" className="h-7 px-3">
                Translate
              </TabsTrigger>
              <TabsTrigger value="improve" className="h-7 px-3">
                Improve
              </TabsTrigger>
            </TabsList>
          </Tabs>

          <div className="ml-auto flex items-center gap-1">
            <Tooltip>
              <TooltipTrigger
                render={
                  <Button
                    variant="ghost"
                    size="icon"
                    aria-label="History"
                    onClick={onOpenHistory}
                  />
                }
              >
                <HistoryIcon />
              </TooltipTrigger>
              <TooltipContent>History</TooltipContent>
            </Tooltip>

            <Tooltip>
              <TooltipTrigger
                render={
                  <Button
                    variant="ghost"
                    size="icon"
                    aria-label="Settings"
                    onClick={onOpenSettings}
                  />
                }
              >
                <SettingsIcon />
              </TooltipTrigger>
              <TooltipContent>Settings</TooltipContent>
            </Tooltip>
          </div>
        </header>

        <div className="flex min-h-0 flex-1 flex-col">{children}</div>

        <footer className="flex h-9 shrink-0 items-center gap-5 border-t border-border/60 px-4 text-xs text-muted-foreground">
          <span className="inline-flex items-center gap-1.5">
            <Kbd className="bg-muted/60 text-[0.6875rem]">
              {translateShortcut}
            </Kbd>
            Translate
          </span>
          <span className="inline-flex items-center gap-1.5">
            <Kbd className="bg-muted/60 text-[0.6875rem]">{replaceShortcut}</Kbd>
            Replace
          </span>
        </footer>
      </div>
    </TooltipProvider>
  );
}
