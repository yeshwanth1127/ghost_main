import { useState, useEffect } from "react";
import {
  getGlobalStats,
  getModelBreakdown,
  getTopUsers,
  getRecentMessages,
  GlobalStats,
  ModelBreakdownRow,
  TopUserRow,
  RecentMessageRow,
} from "../api";

function Card({ title, value }: { title: string; value: string | number }) {
  return (
    <div className="rounded-lg border border-white/20 bg-white/5 px-4 py-3 sm:px-6 sm:py-4">
      <div className="mb-1 text-xs" style={{ color: "#c96a5b" }}>{title}</div>
      <div className="text-lg sm:text-2xl font-semibold text-white truncate">{value}</div>
    </div>
  );
}

function formatDateTime(value: unknown): string {
  if (value == null) return "";
  const str = String(value);
  try {
    const d = new Date(str);
    if (isNaN(d.getTime())) return str;
    return d.toLocaleString(undefined, {
      dateStyle: "medium",
      timeStyle: "medium",
    });
  } catch {
    return str;
  }
}

type TableColumn<T> = { key: keyof T; label: string; format?: (v: unknown) => string };

function Table<T extends object>({
  columns,
  data,
  rowKey,
}: {
  columns: TableColumn<T>[];
  data: T[];
  rowKey: (row: T) => string;
}) {
  return (
    <table className="w-full border-collapse">
      <thead>
        <tr className="border-b border-white/20">
          {columns.map((c) => (
            <th
              key={String(c.key)}
              className="px-3 py-2 sm:px-4 sm:py-3 text-left text-xs font-medium whitespace-nowrap"
              style={{ color: "#c96a5b" }}
            >
              {c.label}
            </th>
          ))}
        </tr>
      </thead>
      <tbody>
        {data.map((row) => (
          <tr key={rowKey(row)} className="border-b border-white/10">
            {columns.map((c) => {
              const raw = (row as Record<string, unknown>)[c.key as string];
              const display = c.format ? c.format(raw) : String(raw ?? "");
              return (
                <td key={String(c.key)} className="px-3 py-2 sm:px-4 sm:py-3 text-xs sm:text-sm text-white whitespace-nowrap">
                  {display}
                </td>
              );
            })}
          </tr>
        ))}
      </tbody>
    </table>
  );
}

export default function Dashboard() {
  const [stats, setStats] = useState<GlobalStats | null>(null);
  const [modelBreakdown, setModelBreakdown] = useState<ModelBreakdownRow[]>([]);
  const [topUsers, setTopUsers] = useState<TopUserRow[]>([]);
  const [recentMessages, setRecentMessages] = useState<RecentMessageRow[]>([]);
  const [error, setError] = useState("");
  const [loading, setLoading] = useState(true);

  useEffect(() => {
    const load = async () => {
      try {
        const [s, m, t, r] = await Promise.all([
          getGlobalStats(),
          getModelBreakdown(),
          getTopUsers(),
          getRecentMessages(),
        ]);
        setStats(s);
        setModelBreakdown(m);
        setTopUsers(t);
        setRecentMessages(r);
      } catch (err) {
        setError(err instanceof Error ? err.message : "Failed to load");
      } finally {
        setLoading(false);
      }
    };
    load();
  }, []);

  const handleLogout = () => {
    localStorage.removeItem("admin_token");
    window.location.href = "/login";
  };

  if (loading) {
    return (
      <div className="flex min-h-screen items-center justify-center px-4 py-12 sm:px-6 lg:px-12 xl:px-32 text-white/80" style={{ fontFamily: "Space Grotesk, sans-serif" }}>
        Loading...
      </div>
    );
  }

  if (error) {
    return (
      <div className="px-4 py-12 sm:px-6 sm:py-16 md:px-8 lg:px-12 xl:px-32" style={{ fontFamily: "Space Grotesk, sans-serif" }}>
        <span className="text-red-400">{error}</span>
        <button
          onClick={() => window.location.reload()}
          className="ml-4 rounded-lg border border-white bg-black px-4 py-2 text-sm font-medium text-white hover:bg-white/10"
        >
          Retry
        </button>
      </div>
    );
  }

  return (
    <div className="px-12 py-20 lg:px-32 lg:py-24" style={{ fontFamily: "Space Grotesk, sans-serif" }}>
      <div className="max-w-6xl">
        <div className="mb-8 flex flex-col gap-4 sm:flex-row sm:items-center sm:justify-between">
          <h1 className="text-2xl font-bold" style={{ fontFamily: "Bebas Neue, sans-serif", color: "#ff9a8b" }}>
            Ghost Admin Dashboard
          </h1>
          <button
            onClick={handleLogout}
            className="rounded-lg border border-white/30 bg-transparent px-4 py-2 text-sm font-medium text-white/80 transition-colors hover:bg-white/10 hover:text-white"
          >
            Logout
          </button>
        </div>

        <div className="mb-6 sm:mb-8 grid gap-3 sm:gap-4 grid-cols-1 sm:grid-cols-2 lg:grid-cols-4">
          <Card title="Total Users" value={stats?.total_users ?? 0} />
          <Card title="Total Tokens" value={stats?.total_tokens?.toLocaleString() ?? 0} />
          <Card title="Total Cost (USD)" value={stats?.total_cost_usd ?? "0"} />
          <Card title="Total Revenue (USD)" value={stats?.total_revenue ?? "0"} />
        </div>

        <section className="mb-6 sm:mb-8">
          <h2 className="mb-3 sm:mb-4 text-sm sm:text-base font-semibold" style={{ color: "#c96a5b" }}>Model Breakdown</h2>
          <div className="overflow-x-auto rounded-lg border border-white/20 bg-white/5">
            <Table
              columns={[
                { key: "model", label: "Model" },
                { key: "provider", label: "Provider" },
                { key: "tokens", label: "Tokens" },
                { key: "cost_usd", label: "Cost USD" },
                { key: "requests", label: "Requests" },
              ]}
              data={modelBreakdown}
              rowKey={(r) => `${r.model}-${r.provider}`}
            />
          </div>
        </section>

        <section className="mb-6 sm:mb-8">
          <h2 className="mb-3 sm:mb-4 text-sm sm:text-base font-semibold" style={{ color: "#c96a5b" }}>Top Users</h2>
          <div className="overflow-x-auto rounded-lg border border-white/20 bg-white/5">
            <Table
              columns={[
                { key: "email", label: "Email" },
                { key: "tokens", label: "Tokens" },
                { key: "cost_usd", label: "Cost USD" },
              ]}
              data={topUsers}
              rowKey={(r) => `${r.email ?? "null"}-${r.tokens}`}
            />
          </div>
        </section>

        <section>
          <h2 className="mb-4 text-base font-semibold" style={{ color: "#c96a5b" }}>Recent Messages</h2>
          <div className="max-h-[400px] overflow-y-auto overflow-hidden rounded-lg border border-white/20 bg-white/5">
            <Table
              columns={[
                { key: "email", label: "User" },
                { key: "model", label: "Model" },
                { key: "provider", label: "Provider" },
                { key: "total_tokens", label: "Tokens" },
                { key: "cost_usd", label: "Cost" },
                { key: "created_at", label: "Date & Time", format: formatDateTime },
              ]}
              data={recentMessages}
              rowKey={(r) => `${r.user_id}-${r.created_at}`}
            />
          </div>
        </section>
      </div>
    </div>
  );
}
