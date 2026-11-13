/*
 * Copyright (c) 2023 anatawa12 and other contributors
 * SPDX-License-Identifier: MPL-2.0
 *
 * This Source Code Form is subject to the terms of the Mozilla Public
 * License, v. 2.0. If a copy of the MPL was not distributed with this
 * file, You can obtain one at http://mozilla.org.
 */

import i18next from "@/lib/i18n";
import { toastError } from "@/lib/toast";

export function unsupported(feature: string): () => void {
	return () =>
		toastError(i18next.t("general:toast:not supported", { name: feature }));
}
