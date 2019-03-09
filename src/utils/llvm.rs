#[allow(unused_imports)]
use llvm_sys::{
    LLVMModule,LLVMBuilder,LLVMIntPredicate,LLVMLinkage
};

#[allow(unused_imports)]
use llvm_sys::prelude::{
    LLVMValueRef,LLVMTypeRef,LLVMBool,LLVMBasicBlockRef
};

#[allow(unused_imports)]
use llvm_sys::core::{
    LLVMGetTarget,LLVMDisposeModule,LLVMPrintModuleToString,
    LLVMDisposeMessage,LLVMCreatePassManager,LLVMDisposePassManager,
    LLVMRunPassManager,LLVMModuleCreateWithName,LLVMSetTarget,
    LLVMVoidType,LLVMConstInt,LLVMInt8Type,LLVMInt16Type,LLVMBuildPointerCast,
    LLVMInt32Type,LLVMInt1Type,LLVMPointerType,LLVMFunctionType,
    LLVMAddFunction,LLVMCreateBuilder,LLVMPositionBuilderAtEnd,
    LLVMDisposeBuilder,LLVMInt64Type,LLVMDumpModule,LLVMInsertBasicBlock,
    LLVMBuildAlloca,LLVMBuildStore,LLVMBuildGEP,LLVMBuildRet,LLVMInt128Type,
    LLVMBuildBr,LLVMBuildLoad,LLVMBuildICmp,LLVMBuildCondBr,LLVMMDNode,
    LLVMBuildZExt,LLVMBuildIndirectBr,LLVMSetInitializer,LLVMAddDestination,
    LLVMSetGlobalConstant,LLVMSetAlignment,LLVMAddGlobal,LLVMGetParam,
    LLVMArrayType,LLVMConstArray,LLVMBlockAddress,LLVMAppendBasicBlock,
    LLVMCreateFunctionPassManagerForModule,LLVMRunFunctionPassManager,
    LLVMConstString,LLVMSetMetadata,LLVMMDNodeInContext,LLVMGetGlobalContext,
    LLVMMDString,LLVMGetNextBasicBlock,LLVMBuildSwitch,LLVMAddCase,LLVMTypeOf,
    LLVMSetParamAlignment,LLVMBuildLShr,LLVMBuildAnd,LLVMBuildTrunc,
    LLVMSetLinkage,LLVMSetIsInBounds
};

#[allow(unused_imports)]
use llvm_sys::target_machine::{
    LLVMTargetMachineRef,LLVMDisposeTargetMachine,LLVMGetTargetFromTriple,
    LLVMCreateTargetMachine, LLVMCodeGenOptLevel, LLVMRelocMode,
    LLVMCodeModel,LLVMGetDefaultTargetTriple,LLVMCreateTargetDataLayout,
    LLVMTargetMachineEmitToFile,LLVMCodeGenFileType,LLVMAddAnalysisPasses,
    LLVMGetTargetMachineTriple
};

#[allow(unused_imports)]
use llvm_sys::target::{
    LLVM_InitializeAllTargetInfos,LLVM_InitializeAllTargets,
    LLVM_InitializeAllTargetMCs,LLVM_InitializeAllAsmParsers,
    LLVM_InitializeAllAsmPrinters,
    LLVMSetModuleDataLayout
};

#[allow(unused_imports)]
use llvm_sys::transforms::pass_manager_builder::{
    LLVMPassManagerBuilderCreate,LLVMPassManagerBuilderSetOptLevel,
    LLVMPassManagerBuilderPopulateModulePassManager,
    LLVMPassManagerBuilderDispose
};

#[allow(unused_imports)]
use llvm_sys::execution_engine::{
    LLVMExecutionEngineRef,LLVMCreateExecutionEngineForModule,
    LLVMDisposeExecutionEngine,LLVMMCJITCompilerOptions,
    LLVMInitializeMCJITCompilerOptions,
    LLVMCreateMCJITCompilerForModule,LLVMCreateJITCompilerForModule,
    LLVMLinkInMCJIT,
    LLVMCreateInterpreterForModule
};

#[allow(unused_imports)]
use llvm_sys::analysis::{
    LLVMVerifierFailureAction, LLVMVerifyModule
};

#[allow(unused_imports)]
use llvm_sys::execution_engine::{
    LLVMGetFunctionAddress
};

use std::collections::{BTreeSet,BTreeMap};
use std::str;
use std::ffi::{CStr,CString};
use std::ptr::null_mut;
use std::os::raw::{c_ulonglong,c_char,c_uint};
use std::mem;
use std::ptr;

pub const LLVM_FALSE: LLVMBool = 0;
pub const LLVM_TRUE: LLVMBool = 1;

#[allow(dead_code)]
pub fn global_llvm_initialize() {
    // found out it is actually not necessary, but keep it in the module
    // for later use.
    unsafe {
        LLVMLinkInMCJIT();
        LLVM_InitializeAllTargetInfos();
        LLVM_InitializeAllTargets();
        LLVM_InitializeAllTargetMCs();
        LLVM_InitializeAllAsmParsers();
        LLVM_InitializeAllAsmPrinters();
    }
}

pub struct Module {
    pub module: *mut LLVMModule,
    strings: Vec<CString>,
}

impl Drop for Module {
    fn drop(&mut self) {
        // Rust requires that drop() is a safe function.
        unsafe {
            LLVMDisposeModule(self.module);
        }
    }
}

impl Module {
    /// Create a new CString associated with this LLVMModule,
    /// and return a pointer that can be passed to LLVM APIs.
    /// Assumes s is pure-ASCII.
    pub fn new_string_ptr(&mut self, s: &str) -> *const i8 {
        self.new_mut_string_ptr(s)
    }

    // TODO: ideally our pointers wouldn't be mutable.
    pub fn new_mut_string_ptr(&mut self, s: &str) -> *mut i8 {
        let cstring = CString::new(s).unwrap();
        let ptr = cstring.as_ptr() as *mut _;
        self.strings.push(cstring);
        ptr
    }

    #[allow(dead_code)]
    pub fn to_cstring(&self) -> CString {
        unsafe {
            // LLVM gives us a *char pointer, so wrap it in a CStr to mark it
            // as borrowed.
            let llvm_ir_ptr = LLVMPrintModuleToString(self.module);
            let llvm_ir = CStr::from_ptr(llvm_ir_ptr as *const _);

            // Make an owned copy of the string in our memory space.
            let module_string = CString::new(llvm_ir.to_bytes()).unwrap();

            // Cleanup borrowed string.
            LLVMDisposeMessage(llvm_ir_ptr);

            module_string
        }
    }

    #[allow(dead_code)]
    pub fn dump(&mut self) {
        unsafe {
            LLVMDumpModule(self.module);
        }
    }

    #[allow(dead_code)]
    pub fn verify(&self, action: LLVMVerifierFailureAction) -> (bool, String) {
        unsafe {
            let mut err: *mut c_char = ptr::null_mut();

            let flag = LLVMVerifyModule(self.module,
                                        action,
                                        &mut err);


            let errstr = CString::from_raw(err).into_string()
                .expect("UTF-8 Decode Error: error message from LLVMVerifyModule");
            (flag == LLVM_TRUE, errstr)
        }
    }

    pub fn add_global(&self, ty: LLVMTypeRef, name: *const i8) -> LLVMValueRef {
        unsafe {
            LLVMAddGlobal(self.module,
                          ty,
                          name)
        }

    }

    pub fn optimize(&self, func: LLVMValueRef, _tm: &TargetMachine) {
        use llvm_sys::transforms::scalar::*;
        use llvm_sys::transforms::vectorize::*;
        unsafe {
            // http://llvm.org/docs/Frontend/PerformanceTips.html#pass-ordering
            let pass_manager = LLVMCreateFunctionPassManagerForModule(self.module);

            //LLVMAddDemoteMemoryToRegisterPass(pass_manager);
            //LLVMAddAnalysisPasses(tm.tm,pass_manager);
            //LLVMAddBasicAliasAnalysisPass(pass_manager);
            LLVMAddPromoteMemoryToRegisterPass(pass_manager);
            //LLVMAddAlignmentFromAssumptionsPass(pass_manager);
            LLVMAddLoopRotatePass(pass_manager);
            LLVMAddLoopUnswitchPass(pass_manager);
            LLVMAddInstructionCombiningPass(pass_manager);
            LLVMAddIndVarSimplifyPass(pass_manager);
            LLVMAddLoopIdiomPass(pass_manager);
            LLVMAddLoopDeletionPass(pass_manager);
            LLVMAddReassociatePass(pass_manager);
            LLVMAddNewGVNPass(pass_manager);
            LLVMAddJumpThreadingPass(pass_manager);
            //LLVMAddLowerSwitchPass(pass_manager);
            //LLVMAddLoopUnrollPass(pass_manager);
            //LLVMAddLoopRerollPass(pass_manager);
            LLVMAddCFGSimplificationPass(pass_manager);
            LLVMAddAggressiveDCEPass(pass_manager);
            LLVMAddSLPVectorizePass(pass_manager);
            LLVMAddLoopVectorizePass(pass_manager);

            LLVMRunFunctionPassManager(pass_manager, func);
            LLVMRunFunctionPassManager(pass_manager, func);

            LLVMDisposePassManager(pass_manager);
        }
    }

}

pub fn create_module(module_name: &str, target_triple: Option<String>)
                     -> (Module, TargetMachine) {
    let c_module_name = CString::new(module_name).unwrap();
    let module_name_char_ptr = c_module_name.to_bytes_with_nul().as_ptr() as *const _;

    global_llvm_initialize();

    let llvm_module = unsafe {
        LLVMModuleCreateWithName(module_name_char_ptr)
    };

    let module = Module {
        module: llvm_module,
        strings: vec![c_module_name],
    };

    let target_triple_cstring = if let Some(target_triple) = target_triple {
        CString::new(target_triple).unwrap()
    } else {
        get_default_target_triple()
    };

    // This is necessary for maximum LLVM performance, see
    // http://llvm.org/docs/Frontend/PerformanceTips.html
    let tm = unsafe {
        LLVMSetTarget(llvm_module, target_triple_cstring.as_ptr() as *const _);

        let tm = TargetMachine::new(target_triple_cstring.as_ptr() as *const _).unwrap();
        let target_data = LLVMCreateTargetDataLayout(tm.tm);
        LLVMSetModuleDataLayout(module.module, target_data);
        tm
    };

    (module, tm)
}


#[allow(dead_code)]
pub fn int_with_type(ty: LLVMTypeRef, val: c_ulonglong) -> LLVMValueRef {
    unsafe { LLVMConstInt(ty, val, LLVM_FALSE) }
}

/// Convert this integer to LLVM's representation of a constant
/// integer.
#[allow(dead_code)]
pub fn int8(val: c_ulonglong) -> LLVMValueRef {
    unsafe { LLVMConstInt(LLVMInt8Type(), val, LLVM_FALSE) }
}

#[allow(dead_code)]
pub fn int16(val: c_ulonglong) -> LLVMValueRef {
    unsafe { LLVMConstInt(LLVMInt16Type(), val, LLVM_FALSE) }
}

/// Convert this integer to LLVM's representation of a constant
/// integer.
// TODO: this should be a machine word size rather than hard-coding 32-bits.
#[allow(dead_code)]
pub fn int32(val: c_ulonglong) -> LLVMValueRef {
    unsafe { LLVMConstInt(LLVMInt32Type(), val, LLVM_FALSE) }
}

#[allow(dead_code)]
pub fn int64(val: c_ulonglong) -> LLVMValueRef {
    unsafe { LLVMConstInt(LLVMInt64Type(), val, LLVM_FALSE) }
}

#[allow(dead_code)]
pub fn int128(val: c_ulonglong) -> LLVMValueRef {
    unsafe { LLVMConstInt(LLVMInt128Type(), val, LLVM_FALSE) }
}

#[allow(dead_code)]
pub fn const_string(s: *const i8) -> LLVMValueRef {
    unsafe {
        LLVMConstString(s, CStr::from_ptr(s).to_bytes().len() as u32,
                        LLVM_FALSE)
    }
}

pub fn const_array(elem_ty: LLVMTypeRef, vals: &Vec<LLVMValueRef>)
                   -> LLVMValueRef {
    unsafe {
        let ptr = vals.as_ptr();
        LLVMConstArray(elem_ty, mem::transmute(ptr), vals.len() as u32)
    }
}

#[allow(dead_code)]
pub fn int1_type() -> LLVMTypeRef {
    unsafe { LLVMInt1Type() }
}

#[allow(dead_code)]
pub fn int8_type() -> LLVMTypeRef {
    unsafe { LLVMInt8Type() }
}

#[allow(dead_code)]
pub fn int16_type() -> LLVMTypeRef {
    unsafe { LLVMInt16Type() }
}

#[allow(dead_code)]
pub fn int32_type() -> LLVMTypeRef {
    unsafe { LLVMInt32Type() }
}

#[allow(dead_code)]
pub fn int64_type() -> LLVMTypeRef {
    unsafe { LLVMInt64Type() }
}

#[allow(dead_code)]
pub fn int128_type() -> LLVMTypeRef {
    unsafe { LLVMInt128Type() }
}

#[allow(dead_code)]
pub fn int8_ptr_type() -> LLVMTypeRef {
    unsafe { LLVMPointerType(LLVMInt8Type(), 0) }
}
#[allow(dead_code)]
pub fn int16_ptr_type() -> LLVMTypeRef {
    unsafe { LLVMPointerType(LLVMInt16Type(), 0) }
}
#[allow(dead_code)]
pub fn int32_ptr_type() -> LLVMTypeRef {
    unsafe { LLVMPointerType(LLVMInt32Type(), 0) }
}
#[allow(dead_code)]
pub fn int64_ptr_type() -> LLVMTypeRef {
    unsafe { LLVMPointerType(LLVMInt64Type(), 0) }
}
#[allow(dead_code)]
pub fn int128_ptr_type() -> LLVMTypeRef {
    unsafe { LLVMPointerType(LLVMInt128Type(), 0) }
}

pub fn array_type(ty: LLVMTypeRef, size: u32) -> LLVMTypeRef {
    unsafe { LLVMArrayType(ty, size as c_uint) }
}

pub fn add_function(module: &mut Module,
                fn_name: &str,
                args: &mut [LLVMTypeRef],
                ret_type: LLVMTypeRef) -> LLVMValueRef {
    unsafe {
        let fn_type = LLVMFunctionType(ret_type, args.as_mut_ptr(), args.len() as u32, LLVM_FALSE);
        LLVMAddFunction(module.module, module.new_string_ptr(fn_name), fn_type)
    }
}

#[allow(dead_code)]
pub fn insert_basic_block(bb: LLVMBasicBlockRef, name: *const i8)
                          -> LLVMBasicBlockRef {
    unsafe {
        LLVMInsertBasicBlock(bb, name)
    }
}

#[allow(dead_code)]
pub fn insert_basic_block_after(bb: LLVMBasicBlockRef, name: *const i8)
                                -> LLVMBasicBlockRef {
    unsafe {
        LLVMInsertBasicBlock(LLVMGetNextBasicBlock(bb), name)
    }
}

#[allow(dead_code)] // just kept as a future reference
fn add_c_declarations(module: &mut Module) {
    let void;
    unsafe {
        void = LLVMVoidType();
    }

    add_function(module,
                 "llvm.memset.p0i8.i32",
                 &mut [int8_ptr_type(), int8_type(), int32_type(), int32_type(), int1_type()],
                 void);

    add_function(module, "malloc", &mut [int32_type()], int8_ptr_type());

    add_function(module, "free", &mut [int8_ptr_type()], void);

    add_function(module,
                 "write",
                 &mut [int32_type(), int8_ptr_type(), int32_type()],
                 int32_type());

    add_function(module, "putchar", &mut [int32_type()], int32_type());

    add_function(module, "getchar", &mut [], int32_type());
}


pub fn get_default_target_triple() -> CString {
    let target_triple;
    unsafe {
        let target_triple_ptr = LLVMGetDefaultTargetTriple();
        target_triple = CStr::from_ptr(target_triple_ptr as *const _).to_owned();
        LLVMDisposeMessage(target_triple_ptr);
    }

    target_triple
}

#[allow(dead_code)]
pub struct TargetMachine {
    tm: LLVMTargetMachineRef,
}

impl Drop for TargetMachine {
    fn drop(&mut self) {
        unsafe {
            LLVMDisposeTargetMachine(self.tm);
        }
    }
}

impl TargetMachine {
    #[allow(dead_code)]
    pub fn new(target_triple: *const i8) -> Result<Self, String> {
        let mut target = null_mut();
        let mut err_msg_ptr = null_mut();

        unsafe {
            LLVMGetTargetFromTriple(target_triple, &mut target, &mut err_msg_ptr);
            if target.is_null() {
                // LLVM couldn't find a target triple with this name,
                // so it should have given us an error message.
                assert!(!err_msg_ptr.is_null());

                let err_msg_cstr = CStr::from_ptr(err_msg_ptr as *const _);
                let err_msg = str::from_utf8(err_msg_cstr.to_bytes()).unwrap();
                return Err(err_msg.to_owned());
            }
        }

        // cpu is documented: http://llvm.org/docs/CommandGuide/llc.html#cmdoption-mcpu
        let cpu = c_str!("generic");
        // features are documented: http://llvm.org/docs/CommandGuide/llc.html#cmdoption-mattr
        let features = c_str!("");

        let target_machine;
        unsafe {
            target_machine =
                LLVMCreateTargetMachine(target,
                                        target_triple,
                                        cpu,
                                        features,
                                        LLVMCodeGenOptLevel::LLVMCodeGenLevelAggressive,
                                        LLVMRelocMode::LLVMRelocDefault,
                                        LLVMCodeModel::LLVMCodeModelDefault);
            //eprintln!("TARGET = {}", CStr::from_ptr(LLVMGetTargetMachineTriple(target_machine)).to_str().expect("utf8err"));
        }

        Ok(TargetMachine { tm: target_machine })
    }

    pub fn emit_module(&self, module: &Module, path: &str) -> Result<(), String> {
        let mut path_cstr = Vec::new();
        path_cstr.extend_from_slice(path.as_bytes());
        path_cstr.push(b'\0');
        unsafe {
            let mut err: *mut c_char = ptr::null_mut();
            let result = LLVMTargetMachineEmitToFile(
                self.tm,
                module.module,
                path_cstr.as_ptr() as *mut i8,
                LLVMCodeGenFileType::LLVMAssemblyFile,
                &mut err);
            if result != 0 {
                println!("obj_error: {:?}", CStr::from_ptr(err as *const _));
                Err(CStr::from_ptr(err)
                    .to_str()
                    .expect("UTF-8 decoding error (error message in LLVMTargetMachineEmitToFile")
                    .to_string())
            } else {
                Ok(())
            }
        }
    }
}

/// Wraps LLVM's builder class to provide a nicer API and ensure we
/// always dispose correctly.
pub struct Builder {
    pub builder: *mut LLVMBuilder,
}


impl Builder {
    /// Create a new Builder in LLVM's global context.
    #[allow(dead_code)]
    pub fn new() -> Self {
        unsafe { Builder { builder: LLVMCreateBuilder() } }
    }

    #[allow(dead_code)]
    pub fn position_at_end(&self, bb: LLVMBasicBlockRef) {
        unsafe {
            LLVMPositionBuilderAtEnd(self.builder, bb);
        }
    }

    #[allow(dead_code)]
    pub fn alloca(&self, ty: LLVMTypeRef, name: *const i8) -> LLVMValueRef {
        unsafe {
            LLVMBuildAlloca(self.builder, ty, name)
        }
    }

    #[allow(dead_code)]
    pub fn store(&self, val: LLVMValueRef, addr: LLVMValueRef) -> LLVMValueRef {
        unsafe {
            LLVMBuildStore(self.builder, val, addr)
        }
    }

    #[allow(dead_code)]
    pub fn gep(&self, ptr: LLVMValueRef, indices: &Vec<LLVMValueRef>, name: *const i8)
               -> LLVMValueRef {
        // Or, consume indices? it should be safer then.
        unsafe {
            let rawptr = mem::transmute(indices.as_ptr());
            LLVMBuildGEP(self.builder, ptr,
                         rawptr, indices.len() as u32,
                         name)
        }
    }

    #[allow(dead_code)]
    pub fn br(&self, bb: LLVMBasicBlockRef) -> LLVMValueRef {
        unsafe {
            LLVMBuildBr(self.builder, bb)
        }
    }

    #[allow(dead_code)]
    pub fn ret(&self, retval: LLVMValueRef) -> LLVMValueRef {
        unsafe {
            LLVMBuildRet(self.builder, retval)
        }
    }

    #[allow(dead_code)]
    pub fn load(&self, addr: LLVMValueRef, name: *const i8)
                -> LLVMValueRef {
        unsafe {
            LLVMBuildLoad(self.builder, addr,
                          name)
        }
    }

    #[allow(dead_code)]
    pub fn icmp(&self, pred: LLVMIntPredicate,
                lhs: LLVMValueRef, rhs: LLVMValueRef,
                name: *const i8) -> LLVMValueRef {
        unsafe {
            LLVMBuildICmp(self.builder,
                          pred, lhs, rhs, name)
        }
    }

    #[allow(dead_code)]
    pub fn condbr(&self, cond: LLVMValueRef,
                  then: LLVMBasicBlockRef,
                  els: LLVMBasicBlockRef) -> LLVMValueRef {
        unsafe {
            LLVMBuildCondBr(self.builder,
                            cond, then, els)
        }
    }

    #[allow(dead_code)]
    pub fn zext(&self, val: LLVMValueRef, ty: LLVMTypeRef,
                name: *const i8) -> LLVMValueRef {
        unsafe {
            LLVMBuildZExt(self.builder,
                          val, ty, name)
        }
    }

    #[allow(dead_code)]
    pub fn indirect_br(&self, addr: LLVMValueRef, ndest: u32) -> LLVMValueRef {
        unsafe {
            LLVMBuildIndirectBr(self.builder, addr, ndest)
        }
    }

    #[allow(dead_code)]
    pub fn switch(&self, v: LLVMValueRef, els: LLVMBasicBlockRef, num_cases: u32)
                  -> LLVMValueRef {
        unsafe {
            LLVMBuildSwitch(self.builder, v, els, num_cases)
        }
    }

    #[allow(dead_code)]
    pub fn pointer_cast(&self, p: LLVMValueRef, ty: LLVMTypeRef, name: *const i8)
                        -> LLVMValueRef {
        unsafe {
            LLVMBuildPointerCast(self.builder, p, ty, name)
        }
    }

    #[allow(dead_code)]
    pub fn trunc(&self, v: LLVMValueRef, ty: LLVMTypeRef, name: *const i8)
                        -> LLVMValueRef {
        unsafe {
            LLVMBuildTrunc(self.builder, v, ty, name)
        }
    }

    #[allow(dead_code)]
    pub fn and(&self, lhs: LLVMValueRef, rhs: LLVMValueRef, name: *const i8)
               -> LLVMValueRef {
        unsafe {
            LLVMBuildAnd(self.builder, lhs, rhs, name)
        }
    }

    #[allow(dead_code)]
    pub fn lshr(&self, lhs: LLVMValueRef, rhs: LLVMValueRef, name: *const i8)
               -> LLVMValueRef {
        unsafe {
            LLVMBuildLShr(self.builder, lhs, rhs, name)
        }
    }

    // *** Higher-order build functions ***

    #[allow(dead_code)]
    pub fn table_jump(&self, index: LLVMValueRef, lut: &BranchTable,
                      weights: Option<&BTreeMap<LLVMBasicBlockRef, u64>>) {
        let indices = vec![int64(0), index];
        let addr = self.gep(lut.table_ptr, &indices,
                            c_str!(""));
        let addr = self.load(addr, c_str!(""));
        let indbr = self.indirect_br(addr, lut.blocks.len() as u32);

        let dest_set: BTreeSet<_> = lut.blocks.iter().cloned().collect();
        let dest: Vec<_> = dest_set.into_iter().collect();

        for bb in dest.iter() {
            indbr.add_destination(bb.clone())
        }

        match weights {
            Some(weights) => {
                let mut children = vec![
                    unsafe { LLVMMDString(c_str!("branch_weights"), 14) }
                ];
                for bb in dest.iter() {
                    let w = *weights.get(bb)
                        .expect("BasicBlock isn't covered in weight MD");
                    children.push(int32(w as u64));
                }
                let mdnode = create_metadata_node(children);

                indbr.set_metadata(2, mdnode);
            },
            None => {
            }
        };

    }

    #[allow(dead_code)]
    pub fn table_jump_switch(&self, index: LLVMValueRef, lut: &BranchTable,
                             _weights: Option<&BTreeMap<LLVMBasicBlockRef, u64>>) {
        let mut counts: BTreeMap<LLVMBasicBlockRef, u64> = BTreeMap::new();

        for bb in lut.blocks.iter() {
            *counts.entry(*bb).or_insert(0) += 1;
        }

        let (maxbb, maxcnt) = counts.iter().max_by_key(|t| t.1)
            .expect("Must have at least one dest");
        let ncases = lut.blocks.len() - (*maxcnt as usize);

        let switch = self.switch(index, *maxbb, ncases as u32);

        for (v, dest) in lut.blocks.iter().enumerate() {
            if dest == maxbb {
                continue
            };

            let caseval = unsafe {
                LLVMConstInt(LLVMTypeOf(index), v as u64, LLVM_FALSE)
            };

            switch.add_case(caseval, *dest);
        }

    }

}

impl Drop for Builder {
    fn drop(&mut self) {
        // Rust requires that drop() is a safe function.
        unsafe {
            LLVMDisposeBuilder(self.builder);
        }
    }
}

pub struct BranchTable {
    blocks: Vec<LLVMBasicBlockRef>,
    table_ptr: LLVMValueRef,
}

impl BranchTable {
    #[allow(dead_code)]
    pub fn new(module: &Module,
               basefn: LLVMValueRef,
               varname: *const i8,
               bbs: &Vec<LLVMBasicBlockRef>) -> BranchTable {
        let addrs: Vec<_> = {
            bbs.iter().map(|bb| basefn.get_block_address(bb.clone()))
        }.collect();

        let addr_array = const_array(int8_ptr_type(), &addrs);
        let table_val = module.add_global(
                    array_type(int8_ptr_type(), addrs.len() as u32),
                    varname);

        table_val.set_initializer(addr_array);
        table_val.set_global_constant(true);
        table_val.set_alignment(64);

        BranchTable {
            blocks: bbs.clone(),
            table_ptr: table_val
        }
    }
}

pub struct ExecutionEngine {
    pub ee: LLVMExecutionEngineRef
}

impl Drop for ExecutionEngine {
    fn drop(&mut self) {
        unsafe {
            LLVMDisposeExecutionEngine(self.ee);
        }
    }
}

impl ExecutionEngine {
    /// Create a new Builder in LLVM's global context.
    #[allow(dead_code)]
    pub fn new_mcjit(module: Module) -> Self {
        let mut module = module;
        unsafe {
            let mut options: LLVMMCJITCompilerOptions = mem::uninitialized();
            LLVMInitializeMCJITCompilerOptions(&mut options, mem::size_of_val(&options));
            options.OptLevel = 3;
            options.EnableFastISel = LLVM_TRUE;

            let mut ee: LLVMExecutionEngineRef = ptr::null_mut();
            let mut err: *mut c_char = ptr::null_mut();

            LLVMCreateMCJITCompilerForModule(
                &mut ee, module.module,
                &mut options, mem::size_of_val(&options),
                &mut err);

            // Execution engine inherits ownership of module
            // Therefore, LLVMDisposeModule is no longer required.
            module.module = null_mut();

            ExecutionEngine {
                ee: ee
            }
        }
    }

    #[allow(dead_code)]
    pub fn new_old_jit(module: Module) -> Self {
        let mut module = module;
        unsafe {
            let mut ee: LLVMExecutionEngineRef = ptr::null_mut();
            let mut err: *mut c_char = ptr::null_mut();

            LLVMCreateJITCompilerForModule(
                &mut ee, module.module,
                3,
                &mut err);

            // Execution engine inherits ownership of module
            // Therefore, LLVMDisposeModule is no longer required.
            module.module = null_mut();

            ExecutionEngine {
                ee: ee
            }
        }
    }

    #[allow(dead_code)]
    pub fn new_interpreter(module: Module) -> Self {
        let mut module = module;
        unsafe {
            let mut ee: LLVMExecutionEngineRef = ptr::null_mut();
            let mut err: *mut c_char = ptr::null_mut();

            LLVMCreateInterpreterForModule(
                &mut ee, module.module,
                &mut err);


            module.module = null_mut();

            ExecutionEngine {
                ee: ee
            }
        }
    }

    pub fn get_function_addr(&self, name: *const c_char) -> u64 {
        unsafe {
            LLVMGetFunctionAddress(self.ee, name)
        }
    }

}

// TO DO: Wrap values with proper newtypes depending on the type of values
pub trait ValueMethod {
    // About global
    fn set_initializer(self, val: LLVMValueRef);
    fn set_global_constant(self, b: bool);
    fn set_linkage(self, linkage: LLVMLinkage);
    fn set_alignment(self, bytes: u32);

    // about function parameters
    fn set_param_alignment(self, bytes: u32);

    // About function
    fn get_block_address(self, bb: LLVMBasicBlockRef) -> LLVMValueRef;
    fn append_basic_block(self, name: *const i8) -> LLVMBasicBlockRef;
    fn get_param(self, idx: u32) -> LLVMValueRef;

    // about indbr
    fn add_destination(self, bb: LLVMBasicBlockRef);

    // about switch
    fn add_case(self, v: LLVMValueRef, bb: LLVMBasicBlockRef);

    // about gep
    fn set_in_bounds(self, b: bool);

    fn set_metadata(self, kind: u32, val: LLVMValueRef);
}

impl ValueMethod for LLVMValueRef {
    fn set_initializer(self, val: LLVMValueRef) {
        unsafe {
            LLVMSetInitializer(self, val)
        }
    }
    fn set_global_constant(self, b: bool) {
        unsafe {
            LLVMSetGlobalConstant(self, if b { LLVM_TRUE } else { LLVM_FALSE});
        }
    }
    fn set_linkage(self, linkage: LLVMLinkage) {
        unsafe {
            LLVMSetLinkage(self, linkage)
        }
    }


    fn set_param_alignment(self, bytes: u32) {
        unsafe {
            LLVMSetParamAlignment(self, bytes as c_uint);
        }
    }

    fn set_alignment(self, bytes: u32) {
        unsafe {
            LLVMSetAlignment(self, bytes as c_uint);
        }
    }

    fn get_block_address(self, bb: LLVMBasicBlockRef) -> LLVMValueRef {
        unsafe {
            LLVMBlockAddress(self, bb)
        }
    }

    fn append_basic_block(self, name: *const i8) -> LLVMBasicBlockRef {
        unsafe {
            LLVMAppendBasicBlock(self, name)
        }
    }

    fn get_param(self, idx: u32) -> LLVMValueRef {
        unsafe {
            LLVMGetParam(self, idx as c_uint)
        }
    }

    fn add_destination(self, bb: LLVMBasicBlockRef) {
        unsafe {
            LLVMAddDestination(self, bb)
        }
    }

    fn add_case(self, v: LLVMValueRef, bb: LLVMBasicBlockRef) {
        unsafe {
            LLVMAddCase(self, v, bb)
        }
    }

    fn set_in_bounds(self, b: bool) {
        unsafe {
            LLVMSetIsInBounds(self, if b { LLVM_TRUE } else { LLVM_FALSE })
        }
    }

    fn set_metadata(self, kind: u32, val: LLVMValueRef) {
        // For Kind values, see http://llvm.org/doxygen/classllvm_1_1LLVMContext.html
        //http://llvm.org/doxygen/MDBuilder_8h_source.html
        unsafe {
            LLVMSetMetadata(self, kind, val)
        }
    }

}

pub fn create_metadata_node(children: Vec<LLVMValueRef>) -> LLVMValueRef {
    let mut children = children;
    let node = unsafe {
        LLVMMDNode(
            //LLVMGetGlobalContext(),
            children.as_mut_ptr(), children.len() as u32)
    };
    node
}
