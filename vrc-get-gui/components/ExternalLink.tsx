import { ExternalLink as LucideExternalLink } from "lucide-react";
import type React from "react";
import { commands } from "@/lib/bindings";
import { cn } from "@/lib/utils";

export function ExternalLink({
	children,
	className,
	href,
}: {
	children: React.ReactNode;
	className?: string;
	href?: string;
}): React.JSX.Element {
	const body = (
		<>
			{children}
			<LucideExternalLink
				className={"inline ml-1 size-[1.1em] align-text-top"}
			/>
		</>
	);
	if (href) {
		return (
			<a
				className={cn(className, "underline inline")}
				type={"button"}
				href={href}
				onClick={() => commands.utilOpenUrl(href)}
			>
				{body}
			</a>
		);
	} else {
		return <span className={className}>{body}</span>;
	}
}
