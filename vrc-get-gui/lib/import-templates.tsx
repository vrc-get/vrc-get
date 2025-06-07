import { Button } from "@/components/ui/button";
import { Checkbox } from "@/components/ui/checkbox";
import {
	DialogDescription,
	DialogFooter,
	DialogTitle,
} from "@/components/ui/dialog";
import {
	type TauriImportDuplicated,
	type TauriImportTemplateResult,
	commands,
} from "@/lib/bindings";
import { dateToString } from "@/lib/dateToString";
import { type DialogContext, openSingleDialog } from "@/lib/dialog";
import { tc, tt } from "@/lib/i18n";
import { queryClient } from "@/lib/query-client";
import { toastSuccess } from "@/lib/toast";
import i18next from "i18next";
import { useMemo, useState } from "react";

export async function processResult(result: TauriImportTemplateResult) {
	await queryClient.invalidateQueries({
		queryKey: ["environmentProjectCreationInformation"],
	});

	if (result.duplicates.length === 0) {
		// If nothing has duplicated id so we don't need to ask for import
		toastSuccess(
			tc("templates:toast:imported n templates", {
				count: result.imported,
			}),
		);
		return;
	}

	const overrides = await openSingleDialog(AskOverride, {
		templates: result.duplicates,
	});

	if (overrides.length === 0) {
		// If nothing is asked for override, skip calling backend
		toastSuccess(
			tc("templates:toast:imported n templates", {
				count: result.imported,
			}),
		);
		return;
	}

	const overridden =
		await commands.environmentImportTemplateOverride(overrides);

	await queryClient.invalidateQueries({
		queryKey: ["environmentProjectCreationInformation"],
	});

	// Toast with total number
	toastSuccess(
		tc("templates:toast:imported n templates", {
			count: overridden + result.imported,
		}),
	);
}

export function AskOverride({
	dialog,
	templates,
}: {
	templates: TauriImportDuplicated[];
	dialog: DialogContext<TauriImportDuplicated[]>;
}) {
	const [overrides, setOverrides] = useState<TauriImportDuplicated[]>(
		() => templates,
	);

	const format = useMemo(
		() =>
			new Intl.DateTimeFormat(i18next.languages, {
				dateStyle: "short",
				timeStyle: "medium",
			}),
		[],
	);

	return (
		<>
			<DialogTitle>{tc("templates:dialog:duplicated")}</DialogTitle>
			<DialogDescription className={"flex flex-col gap-2"}>
				<p className={"whitespace-normal font-normal"}>
					{tc("templates:dialog:confirm update templates")}
				</p>
				<ul>
					{templates.map((template) => {
						const checked = overrides.includes(template);
						const onChange = (checked: boolean | string) => {
							console.log(`on change: ${checked}`);
							if (checked) {
								setOverrides((existing) =>
									existing.includes(template)
										? existing
										: existing.concat([template]),
								);
							} else {
								setOverrides((existing) =>
									existing.filter((id) => id !== template),
								);
							}
						};

						return (
							<li key={template.id}>
								<label className={"flex items-center gap-2"}>
									<Checkbox checked={checked} onCheckedChange={onChange} />

									<div className={"flex flex-col"}>
										<span>
											{tc(
												template.existing_name === template.importing_name
													? "templates:dialog:template name"
													: "templates:dialog:template name with name change",
												{
													name: template.existing_name,
													new_name: template.importing_name,
												},
											)}
										</span>
										<span>
											{tc("templates:dialog:confirm update information", {
												old_update:
													template.existing_update_date != null
														? dateToString(template.existing_update_date)
														: tt("general:unknown date"),
												new_update:
													template.importing_update_date != null
														? dateToString(template.importing_update_date)
														: tt("general:unknown date"),
											})}
										</span>
									</div>
								</label>
							</li>
						);
					})}
				</ul>
			</DialogDescription>
			<DialogFooter>
				<Button
					onClick={() => {
						dialog.close(overrides);
					}}
					className={"ml-2"}
				>
					{tc("templates:dialog:button:override templates", {
						count: overrides.length,
					})}
				</Button>
			</DialogFooter>
		</>
	);
}
