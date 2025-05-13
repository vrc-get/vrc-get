import * as React from "react";
import { Checkbox } from "@/components/ui/checkbox";
import type { PackageRowInfo } from "@/app/_main/projects/manage/-collect-package-row-info";
import { useTranslation } from "react-i18next";
import { SearchBox } from "@/components/SearchBox";
import { ScrollableCardTable } from "@/components/ScrollableCardTable";

interface PackageMultiSelectProps {
  packages: PackageRowInfo[];
  selected: string[]; // package IDs
  onChange: (selected: string[]) => void;
  cellClassName?: string;
  headClassName?: string;
  checkboxSeparator?: boolean;
}

export function PackageMultiSelect({
  packages,
  selected,
  onChange,
  cellClassName = '',
  headClassName = '',
  checkboxSeparator = false,
}: PackageMultiSelectProps) {
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
  const { t } = useTranslation();

  return (
    <div>
      <div className="mb-3 max-w-xs">
        <SearchBox
          value={search}
          onChange={e => setSearch(e.target.value)}
          className="w-full"
        />
      </div>
      
      <ScrollableCardTable
        className="w-full h-[45vh] rounded-lg shadow-lg bg-card"
        viewportClassName="overflow-x-hidden"
      >
        <thead className="sticky top-0 z-10 bg-card shadow-sm">
          <tr>
            <th className={`${headClassName} py-3 ${checkboxSeparator ? 'border-r border-secondary' : ''} w-12 text-center`}></th>
            <th className={`${headClassName} py-3 pl-6`}>Package</th>
            <th className={`${headClassName} py-3`}>Description</th>
          </tr>
        </thead>
        <tbody>
          {filtered.map((pkg) => (
            <tr key={pkg.id} className="hover:bg-accent/40 transition-colors">
              <td className={`${cellClassName} py-2 ${checkboxSeparator ? 'border-r border-secondary' : ''} w-12 text-center`}>
                <Checkbox
                  checked={selected.includes(pkg.id)}
                  onCheckedChange={() => toggle(pkg.id)}
                />
              </td>
              <td className={`${cellClassName} py-2 pl-6 overflow-hidden max-w-80 text-ellipsis`}>
                <div>
                  <div>{pkg.displayName}</div>
                  <div className="text-xs opacity-60 break-all">{pkg.id}</div>
                </div>
              </td>
              <td className={`${cellClassName} py-2 whitespace-normal break-words`}>{pkg.description}</td>
            </tr>
          ))}
        </tbody>
      </ScrollableCardTable>
    </div>
  );
} 