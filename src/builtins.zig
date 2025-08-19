const arithmetic = @import("builtins/arithmetic.zig");
const control_flow = @import("builtins/control_flow.zig");
const conversion = @import("builtins/conversion.zig");
const data_structures = @import("builtins/data_structures.zig");
const io = @import("builtins/io.zig");
const type_predicates = @import("builtins/type_predicates.zig");
const utility = @import("builtins/utility.zig");
const Vm = @import("Vm.zig");

/// Registers all built-in native functions with the provided Vm.
pub fn registerAll(vm: *Vm) !void {
    try arithmetic.registerAll(vm);
    try control_flow.registerAll(vm);
    try conversion.registerAll(vm);
    try data_structures.registerAll(vm);
    try io.registerAll(vm);
    try type_predicates.registerAll(vm);
    try utility.registerAll(vm);
}
