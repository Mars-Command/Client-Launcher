import { UserRound } from "lucide-react";
import type { CommunityAccount } from "../hooks/useCommunityAccount";
import { AccountPanel } from "./AccountPanel";

export function AccountPage({ account }: { account: CommunityAccount }) {
	return (
		<section aria-labelledby="account-page-title" className="min-h-0 flex-1 overflow-y-auto p-4 sm:p-5">
			<div className="mx-auto w-full max-w-4xl space-y-4">
				<header className="mb-5 flex items-center gap-3 border-b border-white/8 pb-4">
					<span className="grid h-9 w-9 place-items-center rounded-xl border border-cyan-200/15 bg-cyan-300/8 text-cyan-200">
						<UserRound size={17} />
					</span>
					<div>
						<h1 id="account-page-title" className="text-base font-semibold tracking-wide text-slate-100">
							Your Profile
						</h1>
						<p className="mt-0.5 text-[11px] text-slate-500">
							Website identity and desktop authorization
						</p>
					</div>
				</header>
				<AccountPanel account={account} />
			</div>
		</section>
	);
}
