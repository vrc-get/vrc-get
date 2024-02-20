"use client";

import {Card, List, ListItem, ListItemPrefix} from "@material-tailwind/react";
import {Cog6ToothIcon, ListBulletIcon,} from "@heroicons/react/24/solid";

export function SideBar({className}: { className?: string }) {
	"use client"

	return (
		<Card className={`${className} w-auto max-w-[20rem] p-4 shadow-xl shadow-blue-gray-900/5 h-screen shrink-0`}>
			<List>
				<ListItem>
					<ListItemPrefix>
						<ListBulletIcon className="h-5 w-5"/>
					</ListItemPrefix>
					<a href={"/projects"}>Projects</a>
				</ListItem>
				<ListItem>
					<ListItemPrefix>
						<Cog6ToothIcon className="h-5 w-5"/>
					</ListItemPrefix>
					<a href={"/settings"}>Settings</a>
				</ListItem>
			</List>
		</Card>
	);
}
