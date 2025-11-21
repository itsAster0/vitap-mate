<script lang="ts">
    import {
        Card,
        CardHeader,
        CardTitle,
        CardContent,
        CardFooter,
    } from "$lib/components/ui/card";
    import {
        Table,
        TableBody,
        TableCell,
        TableHead,
        TableHeader,
        TableRow,
    } from "$lib/components/ui/table";
    import { Badge } from "$lib/components/ui/badge";
    import {
        Tabs,
        TabsContent,
        TabsList,
        TabsTrigger,
    } from "$lib/components/ui/tabs";
    import Theme from "./theme.svelte";

    export let data;

    const slots = data?.timetable?.slots ?? [];

    const weekdayOrder = ["MON", "TUE", "WED", "THU", "FRI", "SAT", "SUN"];

    const days = weekdayOrder.filter((d) => slots.some((s) => s.day === d));
    let activeTab = days[0];

    const isLab = (t: string) => t.endsWith("LO") || t.endsWith("LA");

    function sortByTime(list) {
        return list.sort((a, b) => a.startTime.localeCompare(b.startTime));
    }
    function to12Hour(time24: any) {
        let [hour, minute, second] = time24.split(":");

        hour = parseInt(hour);

        const ampm = hour >= 12 ? "PM" : "AM";
        const hour12 = hour % 12 || 12;

        return second
            ? `${hour12}:${minute}:${second} ${ampm}`
            : `${hour12}:${minute} ${ampm}`;
    }
    const updateTime = new Date(
        data?.timetable?.updateTime * 1000,
    ).toLocaleString();
    let getSemName = (k: string): string => {
        let map: Map<string, string> = new Map([
            ["AP2025262", "Fall Sem 2025-26"],
        ]);
        if (map.has(k)) {
            return map.get(k) ?? k;
        }
        return k;
    };
</script>

<div class="no-scrollbar">
    <Card class="p-4 space-y-4 w-full max-w-full h-screen ">
        <CardHeader>
            <CardTitle class="text-lg md:text-xl font-semibold">
                <div class="flex items-end justify-evenly">
                    <div class="flex-1">
                        Timetable — {getSemName(data?.timetable?.semesterId)}
                    </div>

                    <Theme />
                </div>
            </CardTitle>
        </CardHeader>

        <CardContent class="w-full  grid-flow-col">
            <Tabs bind:value={activeTab} class="w-full">
                <TabsList
                    class="mb-4 flex   overflow-x-auto  justify-center items-center w-full no-scrollbar"
                >
                    {#each days as day}
                        <TabsTrigger
                            value={day}
                            class="whitespace-nowrap py-1 text-sm md:text-base"
                        >
                            {day}
                        </TabsTrigger>
                    {/each}
                </TabsList>

                {#each days as day}
                    <TabsContent value={day} class="w-full ">
                        <div class="overflow-x-auto no-scrollbar">
                            <Table
                                class="min-w-[600px] md:min-w-0 w-full no-scrollbar"
                            >
                                <TableHeader>
                                    <TableRow>
                                        <TableHead>Time</TableHead>
                                        <TableHead>Course</TableHead>
                                        <TableHead>Code</TableHead>
                                        <TableHead>Room</TableHead>
                                        <TableHead>Type</TableHead>
                                    </TableRow>
                                </TableHeader>

                                <TableBody>
                                    {#each sortByTime(slots.filter((s) => s.day === day)) as slot}
                                        <TableRow>
                                            <TableCell class="font-medium">
                                                {to12Hour(slot.startTime)} – {to12Hour(
                                                    slot.endTime,
                                                )}
                                            </TableCell>

                                            <TableCell>{slot.name}</TableCell>

                                            <TableCell>
                                                <Badge>{slot.courseCode}</Badge>
                                            </TableCell>

                                            <TableCell>{slot.roomNo}</TableCell>

                                            <TableCell>
                                                {#if isLab(slot.courseType)}
                                                    <Badge
                                                        variant="destructive"
                                                    >
                                                        LAB
                                                    </Badge>
                                                {:else}
                                                    <Badge>THEORY</Badge>
                                                {/if}
                                            </TableCell>
                                        </TableRow>
                                    {/each}
                                </TableBody>
                            </Table>
                        </div>
                    </TabsContent>
                {/each}
            </Tabs>
        </CardContent>

        <CardFooter class="text-xs md:text-sm text-muted-foreground">
            Last updated: {updateTime}
        </CardFooter>
    </Card>
</div>
