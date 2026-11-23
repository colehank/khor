import { useEffect, useRef, useState } from "react";
import {
  agentFleet,
  dshSessions,
  openShell,
  type AgentFleet,
  type DeviceRow,
  type DshConversation,
  type FleetAgent,
} from "@/api";
import { askOpenSession } from "@/lib/composer-handoff";
import { DshTranscriptDialog } from "@/components/DshTranscriptDialog";
import { RegionState } from "@/components/RegionState";
import { Button } from "@/components/ui/button";
import { cli, gui } from "@/gen/catalog";
import { MoreSection } from "./MorePage";

/** The one word a row's state wears. */
const stateWord = (row: FleetAgent): string =>
  row.program ? cli.agents_fleet_installed : cli.agents_fleet_absent;

/**
 * Where a row's program is, then where its settings are.
 *
 * Two different facts, and a tool that was uninstalled leaves the second
 * behind — so the path a person is looking for is the first one, and the
 * settings directory is only shown when the program is not there to show.
 */
const where = (row: FleetAgent): string =>
  row.program ?? (row.config ? `${row.config}` : "—");

/** The last segment of a recorded directory, for a row's name. */
const project = (cwd: string): string =>
  cwd.split(/[\\/]/).filter(Boolean).pop() ?? cwd;

/**
 * **What this machine has, not what khor knows about** (docs/WINDOWS…,
 * `khor agents fleet`): each known agent, whether it is installed here,
 * the road khor would take to it, and what that road can do.
 *
 * A row that is not installed stays on the list: it is the answer to
 * "what could I install", and hiding it would make the page a report on
 * this machine alone rather than a map of what to do next.
 *
 * Below the agents, **the conversations DSH already has here** — read from
 * the store it writes, grouped by the directory each was recorded in. Each
 * one can be continued in a terminal; that is the native app owning the
 * conversation, which is the arrangement for native sessions.
 */
export function AgentFleetSection({ devices }: { devices: DeviceRow[] }) {
  const [fleet, setFleet] = useState<AgentFleet | null>(null);
  const [dsh, setDsh] = useState<DshConversation[] | null>(null);
  const [error, setError] = useState<string | null>(null);
  const [refused, setRefused] = useState<Map<string, string>>(new Map());
  /** The conversation the reader is open on, copied when it opened. */
  const [reading, setReading] = useState<DshConversation | null>(null);
  const alive = useRef(true);
  useEffect(() => {
    alive.current = true;
    agentFleet().then(
      (answer) => {
        if (alive.current) setFleet(answer);
      },
      (why) => {
        if (alive.current) setError(String(why));
      },
    );
    // Its own call: a machine without DSH, or a store this cannot read,
    // must not take the agent list down with it.
    dshSessions().then(
      (rows) => {
        if (alive.current) setDsh(rows);
      },
      () => {
        if (alive.current) setDsh([]);
      },
    );
    return () => {
      alive.current = false;
    };
  }, []);
  /** Continue one DSH conversation in a terminal, where DSH owns it. */
  const resumeInTerminal = (conversation: DshConversation) => {
    const me = devices.find((device) => device.me);
    if (!me) return;
    setRefused((prior) => {
      const next = new Map(prior);
      next.delete(conversation.id);
      return next;
    });
    void openShell(
      me.name,
      conversation.cwd,
      // DSH's own resume road: the native TUI adopts the session, and the
      // session's directory is where it must run.
      `dsh tui --resume ${conversation.id}`,
      `DSH · ${project(conversation.cwd)}`,
      crypto.randomUUID(),
    ).then(askOpenSession, (why: unknown) =>
      setRefused((prior) => new Map(prior).set(conversation.id, String((why as Error)?.message ?? why))),
    );
  };
  return (
    <MoreSection name="agent-fleet" label={gui.agent_fleet}>
      <p className="px-row py-in text-callout text-muted-foreground">{gui.agent_fleet_hint}</p>
      {error !== null ? (
        <RegionState state="error" message={error} />
      ) : fleet === null ? (
        <RegionState state="loading-never" />
      ) : (
        <>
          {fleet.runner === null && (
            <p className="px-row py-in text-callout text-muted-foreground">
              {cli.agents_fleet_no_runner}
            </p>
          )}
          {fleet.agents.map((row) => (
            <div
              key={row.id}
              data-setting-row="fact"
              data-setting-row-name={`fleet-${row.id}`}
              data-fleet-agent={row.id}
              data-fleet-state={row.program ? "installed" : "absent"}
              className="flex flex-col gap-0 px-row py-in"
            >
              <span className="flex flex-wrap items-baseline gap-x-in text-body">
                <span>{row.name}</span>
                <span className="text-callout text-secondary-foreground">{stateWord(row)}</span>
                {row.road && <span className="text-callout text-muted-foreground">{row.road}</span>}
              </span>
              <span className="break-words font-mono text-caption text-muted-foreground">
                {where(row)}
              </span>
              {row.capabilities.length > 0 && (
                <span className="text-caption text-muted-foreground">
                  {row.capabilities.join(" · ")}
                </span>
              )}
            </div>
          ))}
        </>
      )}
      {dsh !== null && dsh.length > 0 && (
        <div data-dsh-conversations className="border-t border-separator">
          <p className="px-row py-in text-body">{gui.dsh_conversations}</p>
          {dsh.map((conversation) => (
            <div
              key={conversation.id}
              data-dsh-session={conversation.id}
              className="flex flex-col gap-0 px-row py-in"
            >
              <span className="flex flex-wrap items-baseline gap-x-in text-body">
                <span>{conversation.title ?? project(conversation.cwd)}</span>
                {conversation.parent !== null && (
                  <span className="text-callout text-muted-foreground">↳</span>
                )}
                <Button
                  variant="ghost"
                  data-dsh-resume={conversation.id}
                  onClick={() => resumeInTerminal(conversation)}
                >
                  {gui.resume_native}
                </Button>
                <Button
                  variant="ghost"
                  data-dsh-read-it={conversation.id}
                  onClick={() => setReading(conversation)}
                >
                  {gui.dsh_read}
                </Button>
              </span>
              <span className="break-words font-mono text-caption text-muted-foreground">
                {conversation.cwd}
              </span>
              {refused.has(conversation.id) && (
                <span className="text-callout text-state-failed">{refused.get(conversation.id)}</span>
              )}
            </div>
          ))}
        </div>
      )}
      {/* The reader for one conversation, driven by `reading`: the row it
          was opened from may be gone by the time it closes (a refresh),
          so the id it needs was copied when it opened. */}
      <DshTranscriptDialog
        open={reading !== null}
        id={reading?.id ?? null}
        title={reading ? (reading.title ?? project(reading.cwd)) : ""}
        onClose={() => setReading(null)}
      />
    </MoreSection>
  );
}
