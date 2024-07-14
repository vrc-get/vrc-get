import type {
	TauriBasePackageInfo,
	TauriPackage,
	TauriProjectDetails,
	TauriUserRepository,
	TauriVersion,
} from "@/lib/bindings";
import {
	compareUnityVersion,
	compareVersion,
	toVersionString,
} from "@/lib/version";
import { assertNever } from "@/lib/assert-never";

export type PackageLatestInfo =
	| {
			status: "none";
	  }
	| {
			status: "contains";
			pkg: TauriPackage;
			hasUnityIncompatibleLatest: boolean;
	  }
	| {
			status: "upgradable";
			pkg: TauriPackage;
			hasUnityIncompatibleLatest: boolean;
	  };

export interface PackageRowInfo {
	id: string;
	infoSource: TauriVersion;
	displayName: string;
	description: string;
	aliases: string[];
	unityCompatible: Map<string, TauriPackage>;
	unityIncompatible: Map<string, TauriPackage>;
	sources: Set<string>;
	isThereSource: boolean; // this will be true even if all sources are hidden
	installed: null | {
		version: TauriVersion;
		yanked: boolean;
	};
	latest: PackageLatestInfo;
}

export const VRCSDK_PACKAGES = [
	"com.vrchat.avatars",
	"com.vrchat.worlds",
	"com.vrchat.base",
];

export function combinePackagesAndProjectDetails(
	packages: TauriPackage[],
	project: TauriProjectDetails | null,
	hiddenRepositories?: string[] | null,
	hideLocalUserPackages?: boolean,
	definedRepositories: TauriUserRepository[] = [],
	showPrereleasePackages = false,
): PackageRowInfo[] {
	const hiddenRepositoriesSet = new Set(hiddenRepositories ?? []);

	function isUnityCompatible(
		pkg: TauriPackage,
		unityVersion: [number, number] | null,
	) {
		if (unityVersion == null) return true;
		if (pkg.unity == null) return true;

		// vrcsdk exceptions for unity version
		if (VRCSDK_PACKAGES.includes(pkg.name)) {
			if (pkg.version.major === 3 && pkg.version.minor <= 4) {
				return unityVersion[0] === 2019;
			}
		} else if (pkg.name === "com.vrchat.core.vpm-resolver") {
			if (
				pkg.version.major === 0 &&
				pkg.version.minor === 1 &&
				pkg.version.patch <= 26
			) {
				return unityVersion[0] === 2019;
			}
		}

		return compareUnityVersion(pkg.unity, unityVersion) <= 0;
	}

	const yankedVersions = new Set<`${string}:${string}`>();
	const knownPackages = new Set<string>();
	const packagesPerRepository = new Map<string, TauriPackage[]>();
	const userPackages: TauriPackage[] = [];

	for (const pkg of packages) {
		if (!showPrereleasePackages && pkg.version.pre) continue;

		if (pkg.is_yanked) {
			yankedVersions.add(`${pkg.name}:${toVersionString(pkg.version)}`);
			continue;
		}

		knownPackages.add(pkg.name);

		let packages: TauriPackage[];
		// check the repository is visible
		if (pkg.source === "LocalUser") {
			if (hideLocalUserPackages) continue;
			packages = userPackages;
		} else if ("Remote" in pkg.source) {
			if (hiddenRepositoriesSet.has(pkg.source.Remote.id)) continue;

			packages = packagesPerRepository.get(pkg.source.Remote.id) ?? [];
			packagesPerRepository.set(pkg.source.Remote.id, packages);
		} else {
			assertNever(pkg.source);
		}

		packages.push(pkg);
	}

	const packagesTable = new Map<string, PackageRowInfo>();

	const getRowInfo = (pkg: TauriBasePackageInfo): PackageRowInfo => {
		let packageRowInfo = packagesTable.get(pkg.name);
		if (packageRowInfo == null) {
			packagesTable.set(
				pkg.name,
				(packageRowInfo = {
					id: pkg.name,
					displayName: pkg.display_name ?? pkg.name,
					description: pkg.description ?? "",
					aliases: pkg.aliases,
					infoSource: pkg.version,
					unityCompatible: new Map(),
					unityIncompatible: new Map(),
					sources: new Set(),
					isThereSource: false,
					installed: null,
					latest: { status: "none" },
				}),
			);
		}
		return packageRowInfo;
	};

	function addPackage(pkg: TauriPackage) {
		const packageRowInfo = getRowInfo(pkg);
		packageRowInfo.isThereSource = true;

		if (compareVersion(pkg.version, packageRowInfo.infoSource) > 0) {
			// use display name from the latest version
			packageRowInfo.infoSource = pkg.version;
			packageRowInfo.displayName = pkg.display_name ?? pkg.name;
			packageRowInfo.description =
				pkg.description || packageRowInfo.description;
			packageRowInfo.aliases = pkg.aliases;
		}

		if (project == null || isUnityCompatible(pkg, project.unity)) {
			packageRowInfo.unityCompatible.set(toVersionString(pkg.version), pkg);
		} else {
			packageRowInfo.unityIncompatible.set(toVersionString(pkg.version), pkg);
		}

		if (pkg.source === "LocalUser") {
			packageRowInfo.sources.add("User");
		} else if ("Remote" in pkg.source) {
			packageRowInfo.sources.add(pkg.source.Remote.display_name);
		}
	}

	// predefined repositories
	packagesPerRepository.get("com.vrchat.repos.official")?.forEach(addPackage);
	packagesPerRepository.get("com.vrchat.repos.curated")?.forEach(addPackage);
	userPackages.forEach(addPackage);
	packagesPerRepository.delete("com.vrchat.repos.official");
	packagesPerRepository.delete("com.vrchat.repos.curated");

	// for repositories
	for (const definedRepository of definedRepositories) {
		packagesPerRepository.get(definedRepository.id)?.forEach(addPackage);
		packagesPerRepository.delete(definedRepository.id);
	}

	// in case of repository is not defined
	for (const packages of packagesPerRepository.values()) {
		packages.forEach(addPackage);
	}

	// sort versions
	for (const value of packagesTable.values()) {
		value.unityCompatible = new Map(
			[...value.unityCompatible].sort(
				(a, b) => -compareVersion(a[1].version, b[1].version),
			),
		);
		value.unityIncompatible = new Map(
			[...value.unityIncompatible].sort(
				(a, b) => -compareVersion(a[1].version, b[1].version),
			),
		);
	}

	// set latest info
	for (const value of packagesTable.values()) {
		const latestPackage = value.unityCompatible.values().next().value;
		if (latestPackage) {
			let hasUnityIncompatibleLatest = false;

			const incompatibleLatestPackage = value.unityIncompatible
				.values()
				.next().value;
			if (
				incompatibleLatestPackage &&
				compareVersion(
					latestPackage.version,
					incompatibleLatestPackage.version,
				) < 0
			) {
				hasUnityIncompatibleLatest = true;
			}

			value.latest = {
				status: "contains",
				pkg: latestPackage,
				hasUnityIncompatibleLatest,
			};
		}
	}

	// set installed info
	if (project) {
		for (const [_, pkg] of project.installed_packages) {
			const packageRowInfo = getRowInfo(pkg);

			// if installed, use the installed version to get the display name
			packageRowInfo.displayName = pkg.display_name ?? pkg.name;
			packageRowInfo.aliases = [...pkg.aliases, ...packageRowInfo.aliases];
			packageRowInfo.installed = {
				version: pkg.version,
				yanked:
					pkg.is_yanked ||
					yankedVersions.has(`${pkg.name}:${toVersionString(pkg.version)}`),
			};
			packageRowInfo.isThereSource = knownPackages.has(pkg.name);

			// if we have the latest version, check if it's upgradable
			if (packageRowInfo.latest.status !== "none") {
				const compare = compareVersion(
					pkg.version,
					packageRowInfo.latest.pkg.version,
				);
				if (compare < 0) {
					packageRowInfo.latest = {
						status: "upgradable",
						pkg: packageRowInfo.latest.pkg,
						hasUnityIncompatibleLatest:
							packageRowInfo.latest.hasUnityIncompatibleLatest,
					};
				}
			}
		}
	}

	const isAvatarsSdkInstalled =
		packagesTable.get("com.vrchat.avatars")?.installed != null;
	const isWorldsSdkInstalled =
		packagesTable.get("com.vrchat.worlds")?.installed != null;
	if (isAvatarsSdkInstalled !== isWorldsSdkInstalled) {
		// if either avatars or worlds sdk is installed, remove the packages for the other SDK.

		// collect dependant packages
		const dependantPackages = new Map<string, Set<string>>();
		for (const pkg of packagesTable.values()) {
			if (pkg.latest.status !== "none") {
				for (const dependency of pkg.latest.pkg.vpm_dependencies) {
					let packageInfo = dependantPackages.get(dependency);
					if (packageInfo === undefined) {
						dependantPackages.set(dependency, (packageInfo = new Set()));
					}
					packageInfo.add(pkg.id);
				}
			}
		}

		const toRemove = new Set<string>();

		// remove the other SDK
		if (isAvatarsSdkInstalled) {
			toRemove.add("com.vrchat.worlds");
		} else if (isWorldsSdkInstalled) {
			toRemove.add("com.vrchat.avatars");
		}

		// update forAvatars and forWorlds recursively
		while (toRemove.size > 0) {
			// biome-ignore lint/style/noNonNullAssertion: we know it's not empty
			const pkgId = [...toRemove].pop()!;
			toRemove.delete(pkgId);

			if (!packagesTable.delete(pkgId)) continue; // already removed

			const dependants = dependantPackages.get(pkgId);
			if (dependants != null)
				for (const dependant of dependants) toRemove.add(dependant);
		}
	}

	if (project) {
		for (const [_, pkg] of project.installed_packages) {
			for (const legacyPackage of pkg.legacy_packages) {
				packagesTable.delete(legacyPackage);
			}
		}
	}

	const asArray = Array.from(packagesTable.values());

	// put installed first
	asArray.sort((a, b) => {
		if (a.installed && !b.installed) return -1;
		if (!a.installed && b.installed) return 1;
		return 0;
	});

	return asArray;
}
