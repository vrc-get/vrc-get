import { tc } from "@/lib/i18n";
import Link from "next/link";

type PageType = "/packages/user-packages" | "/packages/repositories";

export function HeadingPageName({
	pageType,
}: {
	pageType: PageType;
}) {
	return (
		<div className={"-ml-1.5"}>
			<div
				className={"grid grid-cols-2 gap-1.5 bg-secondary p-1 -m-1 rounded-md"}
			>
				<HeadingButton
					currentPage={pageType}
					targetPage={"/packages/repositories"}
				>
					{tc("packages:repositories")}
				</HeadingButton>
				<HeadingButton
					currentPage={pageType}
					targetPage={"/packages/user-packages"}
				>
					{tc("packages:user packages")}
				</HeadingButton>
			</div>
		</div>
	);
}

function HeadingButton({
	currentPage,
	targetPage,
	children,
}: {
	currentPage: PageType;
	targetPage: PageType;
	children: React.ReactNode;
}) {
	const button =
		"cursor-pointer py-1.5 font-bold grow-0 hover:bg-background rounded-sm text-center p-2";

	if (currentPage === targetPage) {
		return <div className={`${button} bg-background`}>{children}</div>;
	} else {
		return (
			<Link href={targetPage} className={button}>
				{children}
			</Link>
		);
	}
}
