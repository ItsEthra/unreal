use crate::offset_preset;

offset_preset! {
    pub Default => {
        FNameEntry => {
            Header = 0;
            WideBit = 0;
            LenBit = 6;
        }
        FName => {
            Size = 8;
            Number = 4;
        }
        FProperty => {
            ArrayDim = 0x38;
            ElementSize = 0x3C;
            Flags = 0x40;
            Offset = 0x4C;
            Size = 0x78;
        }
        FField => {
            Class = 0x8;
            Next = 0x20;
            Name = 0x28;
        }
        UFunction => {
            Flags = 0xB0;
            Code = 0xD8;
        }
        UEnum => {
            Names = 0x40;
        }
        UStruct => {
            Super = 0x40;
            Children = 0x48;
            ChildrenProps = 0x50;
            PropsSize = 0x58;
        }
        UField => {
            Next = 0x28;
        }
        UObject => {
            Index = 0xC;
            Class = 0x10;
            Name = 0x18;
            Outer = 0x20;
        }
        FUObjectItem => {
            Size = 0x28;
        }
    }
}
