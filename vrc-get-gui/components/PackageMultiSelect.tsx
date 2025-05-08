import * as React from "react";
import { ScrollableCardTable } from "@/components/ScrollableCardTable";
import { Checkbox } from "@/components/ui/checkbox";
import { Input } from "@/components/ui/input";
import type { PackageRowInfo } from "@/app/_main/projects/manage/-collect-package-row-info";

interface PackageMultiSelectProps {
  packages: PackageRowInfo[];
  selected: string[]; // package IDs
  onChange: (selected: string[]) => void;
  cellClassName?: string;
  headClassName?: string;
  checkboxSeparator?: boolean;
}

export function PackageMultiSelect({ packages, selected, onChange, cellClassName = '', headClassName = '', checkboxSeparator = false }: PackageMultiSelectProps) {
  const [search, setSearch] = React.useState("");

  const filtered = React.useMemo(
    () =>
      packages.filter(
        (pkg) =>
          pkg.displayName.toLowerCase().includes(search.toLowerCase()) ||
          pkg.id.toLowerCase().includes(search.toLowerCase())
      ),
    [packages, search]
  );

  const toggle = (id: string) => {
    if (selected.includes(id)) {
      onChange(selected.filter((x) => x !== id));
    } else {
      onChange([...selected, id]);
    }
  };

  return (
    <div>
      <Input
        placeholder="Search packages..."
        value={search}
        onChange={(e) => setSearch(e.target.value)}
        className="mb-2"
      />
      <ScrollableCardTable className="w-full min-h-[20vh]">
        <thead>
          <tr>
            <th className={headClassName + (checkboxSeparator ? ' border-r border-secondary' : '')}></th>
            <th className={headClassName + ' pl-6'}>Package</th>
            <th className={headClassName}>Description</th>
          </tr>
        </thead>
        <tbody>
          {filtered.map((pkg) => (
            <tr key={pkg.id}>
              <td className={cellClassName + (checkboxSeparator ? ' border-r border-secondary' : '')}>
                <Checkbox
                  checked={selected.includes(pkg.id)}
                  onCheckedChange={() => toggle(pkg.id)}
                />
              </td>
              <td className={cellClassName + ' pl-6'}>
                <div>
                  <div>{pkg.displayName}</div>
                  <div className="text-xs opacity-60">{pkg.id}</div>
                </div>
              </td>
              <td className={cellClassName}>{pkg.description}</td>
            </tr>
          ))}
        </tbody>
      </ScrollableCardTable>
    </div>
  );
} 