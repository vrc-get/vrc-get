"use client"; // Error components must be Client Components
import { useEffect } from "react";
import { commands } from "@/lib/bindings";
import globalInfo from "@/lib/global-info";

export default function ErrorPage({
	error,
}: {
	error: Error;
	reset?: () => void;
}) {
	useEffect(() => {
		console.error(error);
	}, [error]);

	const errorMessage = `${error}`;
	const errorStack = `${error.stack}`;

	const openIssue = () => {
		try {
			const url = new URL("https://github.com/vrc-get/vrc-get/issues/new");
			url.searchParams.append("labels", "bug,vrc-get-gui");
			url.searchParams.append("template", "01_gui_bug-report.yml");
			url.searchParams.append(
				"os",
				`${globalInfo.osInfo} - ${globalInfo.arch}`,
			);
			url.searchParams.append(
				"webview-version",
				`${globalInfo.webviewVersion}`,
			);
			let version = globalInfo.version ?? "unknown";
			if (globalInfo.commitHash) {
				version += ` (${globalInfo.commitHash})`;
			} else {
				version += " (unknown commit)";
			}
			url.searchParams.append("version", version);

			void commands.utilOpenUrl(url.toString());
		} catch (e) {
			console.error(e);
			alert("Failed to open issue page. Please report this bug manually.");
		}
	};

	return (
		<div className={"w-full flex items-center justify-center"}>
			<div
				className={
					"rounded-xl border bg-card text-card-foreground shadow-xs min-w-[50vw] max-w-[100vw] p-4 flex gap-3"
				}
			>
				<div className={"flex flex-col grow overflow-hidden"}>
					<h2>Client-side unrecoverable error occurred</h2>
					<p>This must be a bug! Please report this bug!</p>
					<div>
						<button
							type={"button"}
							className={
								"whitespace-nowrap rounded-md " +
								"text-sm font-medium " +
								"h-10 px-4 py-2 bg-primary text-primary-foreground "
							}
							onClick={openIssue}
						>
							Report Issue
						</button>
					</div>
					<div className={"h-3"} />
					<h3>Error Message:</h3>
					<code className={"whitespace-pre-wrap ml-2 break-words"}>
						{errorMessage}
					</code>
					<h3>Stack Trace:</h3>
					<code className={"whitespace-pre-wrap ml-2 break-words"}>
						{errorStack}
					</code>
				</div>
			</div>
		</div>
	);
}
