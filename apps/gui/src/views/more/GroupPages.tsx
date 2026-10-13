// Three of 设置's seven pages (UX-16, the owner 2026-10-05), each a
// frame around rows that live elsewhere: 终端 (this face's own terminal
// settings and per-machine agent recognition), 会话 (the node's 标签页上限 and 新会话表单, and this face's
// 完成通知) and 工作区 (the node's 单击文件). The node's rows come from
// its table (`ConfigSections`), so these pages keep no list of keys.
import type { DeviceRow } from "@/api";
import { AgentRecognition } from "./AgentRecognition";
import { ConfigSections } from "@/views/more/ConfigRows";
import { FinishedNotifyRow, TerminalSection } from "@/views/more/AppearanceRows";
import { MoreFrame } from "@/views/more/MorePage";

export function TerminalPage({ onClose, devices, machine, onMachine }: { onClose: () => void; devices: DeviceRow[]; machine: string | null; onMachine: (id: string) => void }) {
  return (
    <MoreFrame page="terminal" onClose={onClose}>
      <TerminalSection />
      <AgentRecognition devices={devices} machine={machine} onMachine={onMachine} />
    </MoreFrame>
  );
}

export function SessionPage({ onClose, onSettingChanged }: { onClose: () => void; onSettingChanged: () => void }) {
  return (
    <MoreFrame page="session" onClose={onClose}>
      <ConfigSections page="session" onChanged={onSettingChanged} after={{ session: <FinishedNotifyRow /> }} />
    </MoreFrame>
  );
}

export function WorkspacePage({ onClose, onSettingChanged }: { onClose: () => void; onSettingChanged: () => void }) {
  return (
    <MoreFrame page="workspace" onClose={onClose}>
      <ConfigSections page="workspace" onChanged={onSettingChanged} />
    </MoreFrame>
  );
}
