import * as React from "react";
import { Checkbox } from "@/components/ui/checkbox";
import type { PackageRowInfo } from "@/app/_main/projects/manage/-collect-package-row-info";
import { useTranslation } from "react-i18next";
import { SearchBox } from "@/components/SearchBox";
import { ScrollableCardTable } from "@/components/ScrollableCardTable";
import { toVersionString } from "@/lib/version";
import { Select, SelectContent, SelectItem, SelectTrigger, SelectValue } from "@/components/ui/select";
import type { TauriPackage } from "@/lib/bindings";

interface PackageMultiSelectProps {
  packages: PackageRowInfo[];
  selected: Record<string, string>;
  onChange: (selected: Record<string, string>) => void;
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

  const handleCheckChange = (pkgId: string, isChecked: boolean) => {
    const newSelected = { ...selected };
    if (isChecked) {
      const packageInfo = packages.find(p => p.id === pkgId);
      let defaultVersionString = "*";

      if (packageInfo) {
        const compatibleVersions = Array.from(packageInfo.unityCompatible.values());
        let stableCandidate: TauriPackage | undefined = undefined;

        // Check if a stable version exists and is compatible
        if (packageInfo.stableLatest.status === "contains" || packageInfo.stableLatest.status === "upgradable") {
          const stableVersion = toVersionString(packageInfo.stableLatest.pkg.version);
          stableCandidate = compatibleVersions.find(cv => toVersionString(cv.version) === stableVersion);
        }

        if (stableCandidate) {
          defaultVersionString = toVersionString(stableCandidate.version);
        } else {
          // Fallback to the absolute latest in the compatible list (newest first)
          if (compatibleVersions.length > 0) {
            defaultVersionString = toVersionString(compatibleVersions[0].version);
          }
          // If compatibleVersions is empty, it remains "*", which is fine.
        }
      }
      newSelected[pkgId] = defaultVersionString;
    } else {
      delete newSelected[pkgId];
    }
    onChange(newSelected);
  };

  const handleVersionChange = (pkgId: string, version: string) => {
    const newSelected = { ...selected, [pkgId]: version };
    onChange(newSelected);
  };

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
        className="w-full h-72 rounded-lg shadow-lg bg-card"
        viewportClassName="overflow-x-hidden"
      >
        <thead className="sticky top-0 z-10 bg-card shadow-sm">
          <tr>
            <th className={`${headClassName} py-3 ${checkboxSeparator ? 'border-r border-secondary' : ''} w-12 text-center`}></th>
            <th className={`${headClassName} py-3 pl-6`}>Package</th>
            <th className={`${headClassName} py-3 w-32 max-w-[8rem] text-center px-2`}>Version</th>
          </tr>
        </thead>
        <tbody>
          {filtered.map((pkg) => {
            const isChecked = pkg.id in selected;
            const availableVersions = Array.from(pkg.unityCompatible.values());

            return (
              <tr key={pkg.id} className="hover:bg-accent/40 transition-colors">
                <td className={`${cellClassName} py-2 ${checkboxSeparator ? 'border-r border-secondary' : ''} w-12 text-center`}>
                  <Checkbox
                    checked={isChecked}
                    onCheckedChange={(checkedState) => handleCheckChange(pkg.id, !!checkedState)}
                  />
                </td>
                <td className={`${cellClassName} py-2 pl-6 overflow-hidden max-w-80 text-ellipsis`}>
                  <div>
                    <div>{pkg.displayName}</div>
                    <div className="text-xs opacity-60 break-all">{pkg.id}</div>
                  </div>
                </td>
                <td className={`${cellClassName} py-2 w-32 max-w-[8rem] overflow-hidden px-2`}>
                  <div className="flex w-full justify-center">
                    {isChecked ? (
                      <Select
                        value={selected[pkg.id]}
                        onValueChange={(version) => handleVersionChange(pkg.id, version)}
                      >
                        <SelectTrigger className="h-8 w-fit min-w-[5rem] text-sm relative pl-2 pr-7 flex justify-center items-center box-border [&>:last-child]:absolute [&>:last-child]:right-1 [&>:last-child]:top-1/2 [&>:last-child]:-translate-y-1/2">
                          <SelectValue placeholder="Select version" />
                        </SelectTrigger>
                        <SelectContent
                          className="w-32 max-w-[8rem]"
                          align="start"
                          avoidCollisions={false}
                          position="popper"
                          side="bottom"
                          sideOffset={4}
                        >
                          {availableVersions.length > 0 ? (
                            availableVersions.map(vPkg => (
                              <SelectItem key={toVersionString(vPkg.version)} value={toVersionString(vPkg.version)} className="text-sm">
                                {toVersionString(vPkg.version)}
                              </SelectItem>
                            ))
                          ) : (
                            <SelectItem value="-" disabled>
                              No versions available
                            </SelectItem>
                          )}
                        </SelectContent>
                      </Select>
                    ) : (
                      <span className="block mx-auto text-center text-sm truncate">
                        {(pkg.latest.status === "contains" || pkg.latest.status === "upgradable" 
                          ? toVersionString(pkg.latest.pkg.version) 
                          : pkg.installed
                            ? toVersionString(pkg.installed.version)
                            : "-")}
                      </span>
                    )}
                  </div>
                </td>
              </tr>
            );
          })}
        </tbody>
      </ScrollableCardTable>
    </div>
  );
} 