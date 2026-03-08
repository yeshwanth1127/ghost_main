import { Outlet } from "react-router-dom";
import SideButtons from "./SideButtons";

export default function Layout() {
  return (
    <div className="min-h-screen bg-black">
      <main className="pb-24 sm:pb-28 lg:pb-0 lg:pr-48 xl:pr-64">
        <Outlet />
      </main>
      <SideButtons />
    </div>
  );
}
