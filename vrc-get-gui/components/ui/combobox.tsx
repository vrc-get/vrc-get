"use client"

import * as React from "react"
import { Check, ChevronDown } from "lucide-react"

import { cn } from "@/lib/utils"
import {
  Command,
  CommandEmpty,
  CommandGroup,
  CommandItem,
  CommandList,
} from "@/components/ui/command"
import {
  Popover,
  PopoverContent,
  PopoverTrigger,
} from "@/components/ui/popover"
import { Input } from "@/components/ui/input"

export interface ComboboxOption {
  value: string;
  label: string;
  isInstalled?: boolean;
  isSupported?: boolean;
}

/** The _trigger_ is now a real <Input>, fully editable. */
interface ComboboxProps {
  options: ComboboxOption[];
  value: string;
  onValueChange: (value: string) => void;
  placeholder?: string;
  searchPlaceholder?: string;   // kept for API parity
  emptyStateMessage?: string;
  className?: string;
}

export function Combobox({
  options,
  value,
  onValueChange,
  placeholder = ">=2022 * =2022.3.22",
  emptyStateMessage = "No versions match.",
  className,
}: ComboboxProps) {
  const [open, setOpen] = React.useState(false);

  /** We keep a local mirror so the user can type freely. */
  const [inputValue, setInputValue] = React.useState(value);

  /** Whenever the external value changes, sync the field. */
  React.useEffect(() => setInputValue(value), [value]);

  const inputRef = React.useRef<HTMLInputElement>(null);
  const containerRef = React.useRef<HTMLDivElement>(null);

  // Helper to parse version string into numeric tuple [major, minor, patch]
  const parseVersion = (v: string): [number, number, number] | null => {
    if (!v) return null;
    const segs = v.split(".");
    if (segs.length === 0 || segs.length > 3) return null;
    const nums: number[] = segs.map(s => parseInt(s, 10));
    if (nums.some(n => isNaN(n))) return null;
    while (nums.length < 3) nums.push(0); // pad minor/patch with 0
    return [nums[0], nums[1], nums[2]];
  };

  const compare = (a: [number, number, number], b: [number, number, number]) => {
    if (a[0] !== b[0]) return a[0] - b[0];
    if (a[1] !== b[1]) return a[1] - b[1];
    return a[2] - b[2];
  };

  const matchesComparator = (pattern: string, version: string): boolean => {
    const trimmed = pattern.trim();
    const opMatch = trimmed.match(/^(>=|<=|>|<|=)\s*(.*)$/);
    if (!opMatch) return false;
    const [, op, rhsRaw] = opMatch;
    if (!rhsRaw) return true; // empty RHS, treat as show all
    const rhsVer = parseVersion(rhsRaw);
    const lhsVer = parseVersion(version);
    if (!rhsVer || !lhsVer) return false;
    const cmp = compare(lhsVer, rhsVer);
    switch (op) {
      case ">":
        return cmp > 0;
      case ">=":
        return cmp >= 0;
      case "<":
        return cmp < 0;
      case "<=":
        return cmp <= 0;
      case "=":
        return cmp === 0;
      default:
        return false;
    }
  };

  const getMatchingVersions = (input: string): ComboboxOption[] => {
    const raw = input.trim();
    if (raw === "") return options; // empty => show all

    // comparator patterns
    if (/^(>=|<=|>|<|=)/.test(raw)) {
      return options.filter(o => matchesComparator(raw, o.value));
    }

    // wildcard * or x pattern (e.g., 2022.*  or 2022.x)
    if (/[\*x]/.test(raw)) {
      const regex = new RegExp("^" + raw.replace(/\./g, "\\.").replace(/\*/g, "\\d+").replace(/x/g, "\\d+") + "$", "i");
      return options.filter(o => regex.test(o.value));
    }

    // numeric prefix search (e.g., '202' should match 2022.3.22)
    if (/^\d+$/.test(raw)) {
      return options.filter(o => o.value.startsWith(raw));
    }

    // fallback: substring case-insensitive match
    return options.filter(o => (o.label ?? o.value).toLowerCase().includes(raw.toLowerCase()));
  };

  /** Filter dropdown as user types. */
  const filtered = React.useMemo(() => {
    // If the current value exactly matches one option, still show full list but with that option first
    const exact = options.find(o => o.value === inputValue);
    let list: ComboboxOption[];
    if (exact) {
      list = options;
    } else {
      list = getMatchingVersions(inputValue);
    }
    return list;
  }, [options, inputValue]);

  // Separate installed and supported-only versions for visual grouping
  const installedVersions = React.useMemo(() => 
    filtered.filter(o => o.isInstalled), 
    [filtered]
  );
  
  const supportedVersions = React.useMemo(() => 
    filtered.filter(o => !o.isInstalled && o.isSupported), 
    [filtered]
  );

  return (
    <div className="relative w-full" ref={containerRef}>
      <Popover open={open} onOpenChange={setOpen}>
        <PopoverTrigger asChild>
          <div className="relative w-full">
            <Input
              ref={inputRef}
              role="combobox"
              aria-expanded={open}
              aria-controls="unity-version-list"
              placeholder={placeholder}
              className={cn("pr-10", className)}
              value={inputValue}
              onChange={(e) => {
                const v = e.target.value;
                setInputValue(v);
                onValueChange(v);       // propagate free-text
                if (!open) setOpen(true);
              }}
              onKeyDown={(e) => {
                // Down arrow opens the list & focuses first item
                if (e.key === "ArrowDown" && !open) {
                  e.preventDefault();
                  setOpen(true);
                }
                // Escape closes the list but keeps the text
                if (e.key === "Escape" && open) {
                  e.preventDefault();
                  setOpen(false);
                }
              }}
            />
            <ChevronDown className="absolute right-3 top-3 h-4 w-4 opacity-50 pointer-events-none" />
          </div>
        </PopoverTrigger>

        <PopoverContent
          className="p-0 min-w-[300px] w-full"
          onOpenAutoFocus={(e) => e.preventDefault()} // keep outer input focused
          sideOffset={4}
          align="start"
        >
          <Command shouldFilter={false}>
            {/* We **don't** render CommandInput – the outer Input drives filtering */}
            <CommandList id="unity-version-list" aria-label="Unity versions" className="max-h-[200px] overflow-auto">
              {filtered.length === 0 && (
                <CommandEmpty>{emptyStateMessage}</CommandEmpty>
              )}
              
              {installedVersions.length > 0 && (
                <CommandGroup heading="Installed Versions">
                  {installedVersions.map((option) => (
                    <CommandItem
                      key={option.value}
                      value={option.value}
                      onSelect={(v) => {
                        onValueChange(v);
                        setInputValue(v);
                        setOpen(false);
                      }}
                      className="flex items-center"
                    >
                      <Check
                        className="mr-2 h-4 w-4 shrink-0 text-green-500"
                      />
                      {option.label}
                    </CommandItem>
                  ))}
                </CommandGroup>
              )}
              
              {supportedVersions.length > 0 && (
                <CommandGroup heading="Supported Versions">
                  {supportedVersions.map((option) => (
                    <CommandItem
                      key={option.value}
                      value={option.value}
                      onSelect={(v) => {
                        onValueChange(v);
                        setInputValue(v);
                        setOpen(false);
                      }}
                      className="flex items-center pl-8"
                    >
                      {option.label}
                    </CommandItem>
                  ))}
                </CommandGroup>
              )}
              
              {/* Show any remaining options that matched but aren't installed or explicitly supported */}
              {filtered.length > (installedVersions.length + supportedVersions.length) && (
                <CommandGroup heading="Other Versions">
                  {filtered
                    .filter(o => !o.isInstalled && !o.isSupported)
                    .map((option) => (
                      <CommandItem
                        key={option.value}
                        value={option.value}
                        onSelect={(v) => {
                          onValueChange(v);
                          setInputValue(v);
                          setOpen(false);
                        }}
                        className="flex items-center pl-8 opacity-80"
                      >
                        {option.label}
                      </CommandItem>
                    ))}
                </CommandGroup>
              )}
            </CommandList>
          </Command>
        </PopoverContent>
      </Popover>
    </div>
  );
}
