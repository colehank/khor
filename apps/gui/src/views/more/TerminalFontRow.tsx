import {useRef,useState,type ReactNode} from "react";
import {IconChevronDown,IconChevronUp,IconClose} from "@/components/icons";
import {DropdownMenu,DropdownMenuContent,DropdownMenuItem,DropdownMenuLabel,DropdownMenuSeparator,DropdownMenuTrigger} from "@/components/ui/dropdown-menu";
import {FONT_MAX_MB,addFontFile,fontsHere,removeFont,setTermFont,stackOf,useTermFont} from "@/lib/term-font";
import {gui} from "@/gen/catalog";
import {cn} from "@/lib/utils";
const ROW="flex items-center gap-row px-row py-in";

export function TerminalFontRow() {
  const f=useTermFont();
  const [here]=useState(()=>fontsHere());
  const [refused,setRefused]=useState<string|null>(null);
  const picker=useRef<HTMLInputElement>(null);
  const name=f.choice||gui.term_font_system;
  const pick = (file: File | undefined) => {
    if (!file) return;
    setRefused(null);
    void addFontFile(file).then((why) => {
      if (why === "too-big") setRefused(gui.term_font_too_big(FONT_MAX_MB));
      else if (why === "bad") setRefused(gui.term_font_bad(file.name));
    });
  };
  const item = (family: string, label: string, after?: ReactNode) => (
    <DropdownMenuItem
      key={family || "system"}
      data-term-font-option={family || "system"}
      data-on={f.choice === family || undefined}
      onSelect={() => setTermFont(family)}
      className={cn(f.choice === family && "glass-sel")}
    >
      <span className="min-w-0 flex-1 truncate" style={{ fontFamily: stackOf(family) }}>
        {label}
      </span>
      {after}
    </DropdownMenuItem>
  );
  return <>
        <div data-setting-row="popup" data-setting-row-name="term-font" className={ROW}>
          <span className="min-w-0 flex-1 text-body">{gui.term_font}</span>
          <DropdownMenu>
            <DropdownMenuTrigger asChild>
              <button
                type="button"
                data-term-font
                aria-label={`${gui.term_font}: ${name}`}
                className="-mr-xxs flex h-ctl-md min-w-0 items-center gap-in rounded-sm pr-xxs pl-in text-body hover:bg-fill"
              >
                <span className="min-w-0 truncate">{name}</span>
                <span
                  data-setting-trail
                  className="flex h-4.5 w-4 flex-none flex-col items-center justify-center rounded-xs bg-fill-strong text-muted-foreground"
                >
                  <IconChevronUp centred className="-mb-(--chevron-pair) size-2.5" />
                  <IconChevronDown centred className="size-2.5" />
                </span>
              </button>
            </DropdownMenuTrigger>
            <DropdownMenuContent align="end" data-term-font-menu className="max-h-[60vh] min-w-56 overflow-y-auto">
              {item("", gui.term_font_system)}
              {here.map((n) => item(n, n))}
              {f.added.length > 0 && (
                <>
                  <DropdownMenuSeparator />
                  <DropdownMenuLabel>{gui.term_font_added}</DropdownMenuLabel>
                  {f.added.map((a) =>
                    item(
                      a.family,
                      a.family,
                      <button
                        type="button"
                        data-term-font-remove={a.family}
                        aria-label={`${gui.remove} ${a.family}`}
                        title={gui.remove}
                        onPointerDown={(e) => e.stopPropagation()}
                        onClick={(e) => {
                          e.preventDefault();
                          e.stopPropagation();
                          void removeFont(a.family);
                        }}
                        className="grid size-6 flex-none place-items-center rounded-full text-muted-foreground hover:bg-fill-strong"
                      >
                        <IconClose className="size-3.5" />
                      </button>,
                    ),
                  )}
                </>
              )}
              <DropdownMenuSeparator />
              <DropdownMenuItem data-term-font-add onSelect={() => picker.current?.click()}>
                {gui.term_font_add}
              </DropdownMenuItem>
            </DropdownMenuContent>
          </DropdownMenu>
          <input
            ref={picker}
            type="file"
            hidden
            accept=".ttf,.otf,.woff,.woff2"
            onChange={(e) => {
              pick(e.target.files?.[0]);
              e.target.value = "";
            }}
          />
        </div>
        {refused && (
          <div data-term-font-refused className="px-row text-callout text-destructive">
            {refused}
          </div>
        )}
  </>;
}
