import { type DialogContext, showDialog } from "@/lib/dialog";
import { DialogFooter, DialogTitle } from "@/components/ui/dialog";
import { tc } from "@/lib/i18n";
import { NavigateFn } from "@tanstack/react-router";
import { Button } from "@/components/ui/button";
import { useState } from "react";
import { Input } from "@/components/ui/input";
import { toastError } from "@/lib/toast";

export async function openProjectDetails({
	existingTags,
	existingMemo
}: {
	existingTags: string[],
	existingMemo: string
}, navigate?: NavigateFn) {
	using dialog = showDialog();
	await dialog.ask(openProjectDetailsDialog, { existingTags, existingMemo });
}

function saveProjectDetails({
	newMemo,
	newTags
}:
	{
		newMemo: string,
		newTags: string[]
	}) {
	console.log({
		newMemo,
		newTags
	})
	toastError("Feature is still being developped")
}

function openProjectDetailsDialog({
	dialog,
	existingTags,
	existingMemo
}: {
	dialog: DialogContext<string | null>,
	existingTags: string[],
	existingMemo: string
}) {
	const [modified, setModified] = useState(false)

	const [newMemo, setNewMemo] = useState(existingMemo)
	const [newTags, setNewTags] = useState(existingTags.map(((tag, index) => ({ index, value: tag }))))

	function resetFields() {
		setNewMemo(existingMemo)
		setNewTags(existingTags.map(((tag, index) => ({ index, value: tag }))))
		setModified(false)
	}

	return (<>
		<DialogTitle>{tc("projects:menuitem:project details")}</DialogTitle>
		<section>
			<h3 className="mb-1">
				Memo
			</h3>
			<Input className="w-full min-h-5" value={newMemo} placeholder={newMemo} onChange={(changeEvent => {
				setModified(true)
				setNewMemo(changeEvent.target.value)
			})} />
		</section>
		<section className="mb-2">
			<h3 className="mb-1">
				Tags
			</h3>
			<ul>
				{newTags.map((newTag) => (<li>
					<Input
						key={newTag.index}
						className="w-full mb-1"
						value={newTag.value}
						placeholder={newTag.value}
						onChange={(updateEvent) => {
							setModified(true)
							setNewTags(newTags.map(tag => {
								if (tag.index === newTag.index)
									tag.value = updateEvent.target.value
								return tag
							}))
						}}
					/>
				</li>))}
			</ul>
		</section>
		<DialogFooter className={"gap-2"}>
			<Button disabled={!modified} onClick={resetFields}>
				{tc("general:button:reset")}
			</Button>
			<Button onClick={() => dialog.close(null)}>
				{tc("general:button:cancel")}
			</Button>
			{/* button save not working */}
			<Button
				disabled={!modified}
				onClick={() => {
					saveProjectDetails({
						newMemo,
						newTags: newTags.map(tag => tag.value)
					})
				}}
			>
				{tc("project details:buttons:confirm")}
			</Button>
		</DialogFooter>
	</>)
}
