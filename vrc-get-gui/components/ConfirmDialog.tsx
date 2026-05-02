import React from "react";
import { Button } from "@/components/ui/button";
import { DialogFooter, DialogTitle } from "@/components/ui/dialog";
import type { DialogContext } from "@/lib/dialog";
import { tc } from "@/lib/i18n";

export function ConfirmDialog({
	message,
	dialog,
}: {
	message: React.ReactNode;
	dialog: DialogContext<boolean>;
}) {
	return (
		<>
			<DialogTitle>{tc("general:confirm:refresh during operation")}</DialogTitle>
			<div className="py-4">
				<p>{message}</p>
			</div>
			<DialogFooter>
				<Button variant="outline" onClick={() => dialog.close(false)}>
					{tc("general:button:cancel")}
				</Button>
				<Button variant="destructive" onClick={() => dialog.close(true)}>
					{tc("general:button:continue")}
				</Button>
			</DialogFooter>
		</>
	);
}
